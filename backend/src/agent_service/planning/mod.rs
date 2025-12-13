pub mod prompt;

use crate::agent_service::retry;
use crate::agent_service::util::normalize_yaml_response;
use anyhow::{Context, Result};
use dialect_coach_shared::Dialect;
use dialect_coach_shared::models::plan::import::SimpleImportLanguagePlan;

use crate::agent_service::ocr::OcrService;
use crate::agent_service::provider::{CompletionAgent, CompletionRequest, ProviderAgentConfig};
use std::sync::Arc;

pub struct PlanGenerator {
    agent: Arc<dyn CompletionAgent>,
    config: ProviderAgentConfig,
}

impl PlanGenerator {
    pub fn new(agent: Arc<dyn CompletionAgent>, config: ProviderAgentConfig) -> Self {
        Self { agent, config }
    }

    pub async fn generate_plan_from_images(
        &self,
        images: &[Vec<u8>],
        dialect: Dialect,
    ) -> Result<SimpleImportLanguagePlan> {
        let text = OcrService::transcribe_images(images, &self.config)
            .await
            .context("Failed to transcribe images")?;
        self.generate_plan(&text, dialect).await
    }

    pub async fn generate_plan(
        &self,
        text: &str,
        dialect: Dialect,
    ) -> Result<SimpleImportLanguagePlan> {
        let system_prompt = prompt::build_planning_system_prompt_yaml(dialect);
        let user_prompt = prompt::build_planning_user_prompt_yaml(text);

        let config = crate::agent_service::util::GenerationConfig {
            max_tokens: 4096,
            temperature: 0.1,
        };

        let request = CompletionRequest {
            preamble: &system_prompt,
            prompt: &user_prompt,
            history: &[],
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        };

        // Step 1: Initial (standard) completion call
        let (initial_result, _) =
            retry::retry_completion_call(self.agent.as_ref(), &request, 3).await;

        let response = match initial_result {
            Ok(r) => r,
            Err(e) => return Err(e).context("Failed to generate plan after initial retries"),
        };

        // Step 2: Try parsing
        match try_parse_plan_output_yaml(&response) {
            Ok(plan) => {
                tracing::info!(
                    "Plan generation successful: '{}' ({} steps) for dialect {:?}",
                    plan.title,
                    plan.steps.len(),
                    plan.dialect
                );
                Ok(plan)
            }
            // Step 3: On parse failure, enter tracked retry loop with feedback
            Err(e) => {
                let error_msg = format!("{}", e);
                tracing::warn!(
                    "Plan parsing failed, entering feedback loop. Error: {}",
                    error_msg
                );

                let retry_ctx = retry::RetryContext {
                    agent: self.agent.clone(),
                };

                let preamble_builder = |original: &str, failed: &str, error: &str| {
                    build_retry_planning_preamble_yaml(original, failed, error)
                };

                let retry_params = retry::RetryPromptParams {
                    original_preamble: &system_prompt,
                    failed_response: &response,
                    error_message: &error_msg,
                    prompt: &user_prompt,
                    preamble_builder: &preamble_builder,
                };

                // We need to define these closures here to satisfy the retry interface
                let parse_fn = |resp: &str| try_parse_plan_output_yaml(resp);
                let log_success = |plan: &SimpleImportLanguagePlan| {
                    tracing::info!(
                        "Retry plan generation successful: '{}' ({} steps) for dialect {:?}",
                        plan.title,
                        plan.steps.len(),
                        plan.dialect
                    );
                };

                let (plan, _) = retry_ctx.retry_with_error_feedback_tracked(
                    &retry_params,
                    &[],
                    &config,
                    &parse_fn,
                    &log_success,
                ).await.map_err(|retry_err| {
                    tracing::error!(
                        "Plan generation failed after feedback loop.\nPREAMBLE:\n{}\n\nERROR:\n{:?}",
                        system_prompt,
                        retry_err
                    );
                    retry_err
                })?;

                Ok(plan)
            }
        }
    }
}

pub fn try_parse_plan_output_yaml(response: &str) -> Result<SimpleImportLanguagePlan> {
    let normalized = normalize_yaml_response(response);
    serde_yaml::from_str(&normalized).context("Failed to parse generated plan YAML")
}

pub fn build_retry_planning_preamble_yaml(original: &str, failed: &str, error: &str) -> String {
    retry::build_retry_preamble(
        original,
        failed,
        error,
        "You MUST return valid YAML. Ensure indentation is correct and no markdown fencing is used.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_service::provider::{CompletionAgent, CompletionRequest};
    use async_trait::async_trait;
    use dialect_coach_shared::Dialect;
    use std::sync::Arc;

    struct MockAgent;

    use crate::agent_service::provider::{CompletionAgentError, CompletionOutcome};

    #[async_trait]
    impl CompletionAgent for MockAgent {
        fn provider(&self) -> &'static str {
            "mock"
        }
        fn model(&self) -> &'static str {
            "mock-model"
        }
        async fn completion(
            &self,
            _request: &CompletionRequest<'_>,
        ) -> Result<CompletionOutcome, CompletionAgentError> {
            Ok(CompletionOutcome {
                text: r#"
title: "Test Plan"
dialect: "spanish_mexican"
steps:
  - title: "Step 1"
    instructions: "Learn vocab"
    learning_content:
      - vocab: "hola"
        translation: "hello"
"#
                .to_string(),
                input_tokens: 10,
                output_tokens: 10,
            })
        }
    }

    #[tokio::test]
    async fn test_generator_flow() {
        // Create a mock agent that returns a valid JSON plan
        let mock_agent = Arc::new(MockAgent);
        let config = crate::agent_service::provider::ProviderAgentConfig::openai(
            "test".to_string(),
            None,
            200,
        );
        let generator = PlanGenerator::new(mock_agent, config);

        // Verify public API
        let plan = generator
            .generate_plan("some text", Dialect::SpanishMexican)
            .await
            .unwrap();

        // Asset plan content from mock
        assert_eq!(plan.title, "Test Plan");
        assert_eq!(plan.dialect, Dialect::SpanishMexican);
        assert_eq!(plan.steps.len(), 1);

        let step = &plan.steps[0];
        if let Some(items) = &step.learning_content {
            let item = &items[0];
            assert_eq!(item.vocab.as_deref(), Some("hola"));
            assert_eq!(item.translation.as_deref(), Some("hello"));
        } else {
            panic!("Expected learning content");
        }

        // Verify prompt logic
        let sys = prompt::build_planning_system_prompt_yaml(Dialect::SpanishMexican);
        assert!(sys.contains("CURRICULUM DESIGNER"));

        let usr = prompt::build_planning_user_prompt_yaml("input text");
        assert!(usr.contains("input text"));
    }

    #[test]
    fn test_try_parse_plan_output_yaml_valid() {
        let yaml = r#"
title: "Test Plan"
dialect: "spanish_mexican"
steps:
  - title: "Step 1"
    instructions: "Do this"
    learning_content:
      - vocab: "hola"
        translation: "hello"
"#;
        let plan = try_parse_plan_output_yaml(yaml).unwrap();
        assert_eq!(plan.title, "Test Plan");
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn test_try_parse_plan_output_yaml_fenced() {
        let yaml = r#"
```yaml
title: "Test Plan"
dialect: "spanish_mexican"
steps: []
```
"#;
        let plan = try_parse_plan_output_yaml(yaml).unwrap();
        assert_eq!(plan.title, "Test Plan");
    }

    #[test]
    fn test_try_parse_plan_output_yaml_invalid() {
        let yaml = "invalid: : yaml";
        assert!(try_parse_plan_output_yaml(yaml).is_err());
    }

    #[test]
    fn test_build_retry_planning_preamble_yaml() {
        let original = "Original Preamble";
        let failed = "invalid yaml";
        let error = "some error";
        let preamble = build_retry_planning_preamble_yaml(original, failed, error);
        assert!(preamble.contains(original));
        assert!(preamble.contains("CRITICAL ERROR"));
        assert!(preamble.contains("MUST return valid YAML"));
    }
}
