use super::{
    provider::{ANTHROPIC_PROVIDER, CompletionAgent, CompletionRequest},
    retry::retry_completion_call,
    util::normalize_json_response,
};
use anyhow::{Result, ensure};
use dialect_coach_shared::{ActivityDefinition, AgentUsage, GenerateActivityRequest, LearningGoal};

#[derive(serde::Deserialize)]
struct AuthoredActivity {
    title: String,
    shared_context: String,
    learner_brief: String,
    partner_brief: String,
    goals: Vec<String>,
}

fn parse(text: &str, request: &GenerateActivityRequest) -> Result<ActivityDefinition> {
    let authored: AuthoredActivity = serde_json::from_str(&normalize_json_response(text))?;
    ensure!(
        [
            &authored.title,
            &authored.shared_context,
            &authored.learner_brief,
            &authored.partner_brief
        ]
        .iter()
        .all(|s| !s.trim().is_empty()),
        "Activity text must not be empty"
    );
    ensure!(
        !authored.goals.is_empty() && authored.goals.iter().all(|g| !g.trim().is_empty()),
        "Activity needs nonempty goals"
    );
    Ok(ActivityDefinition {
        activity_id: uuid::Uuid::new_v4(),
        title: authored.title,
        shared_context: authored.shared_context,
        learner_brief: authored.learner_brief,
        partner_brief: authored.partner_brief,
        goals: authored
            .goals
            .into_iter()
            .map(|goal| LearningGoal {
                goal,
                dialect: request.context.dialect,
                completed: None,
            })
            .collect(),
    })
}

pub async fn generate(
    agent: &dyn CompletionAgent,
    request: &GenerateActivityRequest,
) -> (Result<ActivityDefinition>, Vec<AgentUsage>) {
    if request
        .context
        .branch_id
        .is_some_and(|id| id != request.branch_id)
        || request
            .context
            .activity
            .as_ref()
            .is_some_and(|a| a.branch_id != request.branch_id)
    {
        return (
            Err(anyhow::anyhow!("Activity branch identity mismatch")),
            vec![],
        );
    }
    if agent.provider() != ANTHROPIC_PROVIDER {
        return (
            Err(anyhow::anyhow!(
                "Activity creation requires the Anthropic planning channel to transmit temperature 0.7"
            )),
            vec![],
        );
    }
    let context = &request.context;
    let prompt = serde_json::json!({
        "description": request.description, "dialect": context.dialect,
        "script": context.language_option, "formality": context.formality,
        "teaching_mode": context.teaching_mode, "proficiency": context.language_level,
        "plan": context.active_plan, "learning_goals": context.learning_goals,
        "user_gender": context.user_gender,
        "learning_context": { "mistakes": context.past_mistakes, "explained": context.past_explained, "translated": context.past_translated, "exploratory": context.past_exploratory },
        "dialogue": context.context_messages.iter().map(|m| serde_json::json!({"speaker": if m.is_agent() { "partner" } else { "learner" }, "text": m.as_str()})).collect::<Vec<_>>()
    }).to_string();
    let mut preamble = format!(
        "Author a short communicative activity for this conversation. Follow the optional description; otherwise choose from the conversation and settings, including an empty conversation. Use prose, not a fixed recipe catalogue. Normally give one to three concrete observable goals, allowing different successful approaches, without a turn deadline. Distinguish shared context, private learner brief, and hidden partner facts/constraints; partner facts must be usable without the learner's private brief. Keep partner-only facts, values and reasons solely in partner_brief: never reveal them in title, shared_context, learner_brief or goals. Public goals must describe observable learner work, not disclose answers the learner must discover. The app automatically requests a partner opening before the learner sends a message; make both briefs fit that opening. When the task involves discovering information, make partner_brief self-contained about what may be volunteered in the opening and which facts to reveal only in response to relevant learner questions; do not volunteer the answers the learner is meant to discover. State these reveal conditions in ordinary prose without scripting an exchange. Before returning JSON, check each public field for hidden-answer leakage. The learner brief must stand alone and include the shared context needed to begin. Preserve known facts, dialect, script, formality and story continuity. Proficiency is not age. Write learner_brief and every goals string in the target dialect and script at the learner's stated proficiency, or only slightly above it, using the same proficiency guidance below. Use familiar vocabulary, short direct sentences and clear action verbs; allow only occasional unfamiliar language whose meaning is supported by context. Keep the concrete task and outcomes intact: simplify the wording rather than making the activity childish or removing its purpose. Write these as instructions addressed directly to the learner, using language they can already understand, rather than abstract descriptions of learning outcomes. At beginner levels, give one simple action per sentence and split multi-step goals into short sentences. Before returning, read each instruction as the learner and rewrite wording that requires more proficiency than the task itself. Apply the requested script to the instructions themselves as well as to any examples; check every learner-facing field against the script setting. Make the information demands manageable and a successful route feasible under these exact response limits: {}. Internally work through a short successful exchange, but do not output that exchange or prescribe dialogue. Return only JSON with nonempty string fields title, shared_context, learner_brief, partner_brief, and goals as a nonempty array of strings.",
        super::response::activity_level_instruction(context.language_level)
    );
    let mut usage = vec![];
    for attempt in 0..2 {
        let completion = CompletionRequest {
            preamble: &preamble,
            prompt: &prompt,
            history: &[],
            max_tokens: 4096,
            temperature: 0.7,
        };
        let (result, calls) = retry_completion_call(agent, &completion, 3).await;
        usage.extend(calls);
        let text = match result {
            Ok(text) => text,
            Err(error) => return (Err(error), usage),
        };
        let result = parse(&text, request);
        match result {
            Ok(activity) => return (Ok(activity), usage),
            Err(error) if attempt == 0 => preamble.push_str(&format!(
                "\nPrevious attempt failed: {error}. Return the required JSON."
            )),
            Err(error) => return (Err(error), usage),
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_service::provider::{CompletionAgentError, CompletionOutcome};
    use std::sync::Mutex;
    struct Author {
        calls: Mutex<Vec<(f64, String, String)>>,
    }
    #[async_trait::async_trait]
    impl CompletionAgent for Author {
        fn provider(&self) -> &str {
            ANTHROPIC_PROVIDER
        }
        fn model(&self) -> &str {
            "claude-haiku-4-5"
        }
        async fn completion(
            &self,
            request: &CompletionRequest<'_>,
        ) -> Result<CompletionOutcome, CompletionAgentError> {
            let mut calls = self.calls.lock().unwrap();
            calls.push((
                request.temperature,
                request.prompt.to_string(),
                request.preamble.to_string(),
            ));
            Ok(CompletionOutcome {
                text: if calls.len() == 1 {
                    "bad JSON".into()
                } else {
                    r#"{"title":"Choose dinner","shared_context":"Dinner together","learner_brief":"Find a meal","partner_brief":"Only rice and beans remain","goals":["Agree on a meal"]}"#.into()
                },
                input_tokens: 10,
                output_tokens: 5,
            })
        }
    }
    #[tokio::test]
    async fn author_preserves_temperature_description_and_retry_usage() {
        let state = dialect_coach_shared::UserState::new(uuid::Uuid::new_v4());
        let request = GenerateActivityRequest {
            user_id: state.user_id,
            branch_id: state.active_branch_id,
            description: Some("Cook something".into()),
            context: state.build_action_context(vec![]),
        };
        let author = Author {
            calls: Mutex::new(vec![]),
        };
        let (result, usage) = generate(&author, &request).await;
        let activity = result.unwrap();
        assert_eq!(activity.goals[0].dialect, request.context.dialect);
        assert_eq!(activity.goals[0].completed, None);
        assert_eq!(usage.len(), 2);
        assert!(
            author
                .calls
                .lock()
                .unwrap()
                .iter()
                .all(|(temperature, prompt, preamble)| *temperature == 0.7
                    && prompt.contains("Cook something")
                    && preamble.contains("Write learner_brief and every goals string")
                    && preamble.contains(super::super::response::activity_level_instruction(request.context.language_level))
                    && preamble.contains("Keep partner-only facts, values and reasons solely in partner_brief")
                    && preamble.contains("never reveal them in title, shared_context, learner_brief or goals")
                    && preamble.contains("Public goals must describe observable learner work, not disclose answers the learner must discover")
                    && preamble.contains("automatically requests a partner opening before the learner sends a message")
                    && preamble.contains("make partner_brief self-contained about what may be volunteered in the opening")
                    && preamble.contains("reveal only in response to relevant learner questions"))
        );
    }
}
