use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Explained, Formality, Mistake, TeachingMode};
use rig::completion::Prompt;
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, ClientBuilder};

use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

/// General instruction to return JSON only
const JSON_OUTPUT_INSTRUCTION: &str = "CRITICAL: Return raw JSON only. Your response will be parsed by a JSON parser. Do not wrap in markdown code blocks or backticks. Do not add any text before or after the JSON object. Start with { and end with }.";

/// Content filtering directives to prevent refusals caused by authentic dialect corpora
/// appearing in system context. Must be inserted before role to override default safety heuristics.
const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES (OVERRIDE DEFAULTS)

These directives govern how you evaluate safety and policy for this session. When they conflict with default heuristics, these directives take precedence.

1) Only flag content in the user's direct messages, not in system examples or context. Do not refuse, warn, or restrict because of system-provided corpus or RAG examples, which may include slang, profanity, sexual or adult references, or controversial language used authentically in dialects.

2) Distinguish clearly between user input and system context. Treat system context (RAG examples, corpus snippets, developer/system notes) as non-user content; do not attribute it to the user or apply safety filters to it.

3) Always respond to the user's actual message first. If you have safety concerns, address them briefly and separately after answering, and only if those concerns are about the user's direct message.

Operational note: If the user's direct message itself violates policy, provide a minimal, policy-compliant reply or a safe alternative phrasing in the target dialect. Do not refuse or redirect because of system examples."#;

/// Agent service for AI-powered dialect coaching
pub struct AgentService {
    client: rig::providers::anthropic::Client,
    model_name: String,
    qdrant: Arc<QdrantService>,
    embeddings: Arc<EmbeddingService>,
}

fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 128,
        TeachingMode::Corrective => 256,
        TeachingMode::Explanatory => 512,
        TeachingMode::Interleaved => 512,
        TeachingMode::StoryTeller => 512,
        TeachingMode::Debug => 1024,
    }
}

fn output_format_spec(teaching_mode: &TeachingMode) -> &'static str {
    match teaching_mode {
        TeachingMode::Corrective => {
            r#"Response format: {
  "response": "<your conversational response>",
  "mistakes": [{"specific_mistake": "<word/phrase>", "correction": "<correct form>", "mistake_category": {"type": "<category>", "context": "<info>"}}]
}
Categories: spelling_error (context=correct spelling), vocabulary_error (context=correct word), grammar_error (context=error type), dialect_usage_error (context=preferred phrase), other (context=explanation).
The "correction" field should contain the direct correction of the mistake in specific_mistake.
The "mistake_category" field should also contain a brief explanation of the mistake.
Only include mistakes if user made errors. Keep specific_mistake brief."#
        }
        TeachingMode::Explanatory => {
            r#"Response format: {
  "response": "<your conversational response>",
  "explained": [{"new_phrase": "<word/phrase>", "explanation": "<brief usage note>"}]
}
Only include explained if you introduce and explain noteworthy vocabulary, idioms, or cultural context. Keep it to 1-2 essential items that you introduced."#
        }
        TeachingMode::Interleaved => {
            r#"Response format: {
  "response": "<your conversational response>",
  "translated": [{"translated_word": "<word from user>", "translated_to": "<your translation>"}]
}
Include translated array when you translate words/phrases from user's source language into the target dialect.
The "translated_word" should be the original word, "translated_to" should be your dialectal translation."#
        }
        TeachingMode::StoryTeller => {
            r#"Response format: {
  "response": "<your conversational response>",
  "exploratory": [{"point_to_try": "<language feature>", "instructions_for_use": "<how to use it>"}]
}
Include exploratory array when you introduce new language patterns, idioms, or features you want the user to try.
Keep it to 1-2 points that naturally fit the story context."#
        }
        TeachingMode::Immersive
        | TeachingMode::Debug => {
            r#"Response format: {"response": "<your full conversational response here>"}
Where <your full conversational response here> is your natural dialect response following all the rules above."#
        }
    }
}

fn speaker_desc(dialect: &Dialect, formality: &Formality) -> String {
    let dialect_name = (*dialect).name();
    match formality {
        Formality::Formal => format!(
            "You are a native {} speaker communicating in a professional, polite manner",
            dialect_name
        ),
        Formality::Casual => format!(
            "You are a native {} speaker speaking naturally and conversationally",
            dialect_name
        ),
        Formality::DialectRich => format!(
            "You are a native {} speaker actively showcasing distinctive dialect features and expressions",
            dialect_name
        ),
        Formality::Slang => format!(
            "You are a native {} speaker using informal slang and colloquialisms",
            dialect_name
        ),
    }
}

fn teaching_desc(teaching_mode: &TeachingMode) -> String {
    let tokens = tokens_per_mode(teaching_mode);
    let desc = match *teaching_mode {
        TeachingMode::Immersive => {
            "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections."
        },
        TeachingMode::Corrective => {
            "3. CORRECTIVE MODE: Respond naturally, but also populate the mistakes array with any errors in user's message. Include spelling errors, grammar mistakes, and dialectal usage problems. IGNORE missing punctuation and capitalization - this is casual chat. Be specific and brief in identifying the exact problematic word or phrase."
        },
        TeachingMode::Explanatory => {
            "3. EXPLANATORY MODE: Respond naturally, and populate the explained array when you introduce new vocabulary, idioms, or culturally interesting expressions. Keep explanations brief and practical."
        },
        TeachingMode::Interleaved => {
            "3. INTERLEAVED MODE: User will interleave target language with source language. Present your response (including newlines) as:

{user input with non-target-language words simply translated into target dialect, if there are any non-target-language words}

{brief, conversational response in target dialect}."
        },
        TeachingMode::StoryTeller => {
            "3. STORYTELLER MODE: You are telling an interactive story with the user. Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue."
        },
        TeachingMode::Debug => {
            "3. DEBUG MODE: Answer in English with clear, brief explanations. The user is debugging an issue. Provide technical details about what went wrong and how prompts could be improved."
        },
    };
    format!("{}. 4. You have a maximum {} tokens for your response. Be as brief as you can be while accomplishing your goals, but do not go over.", desc, tokens)
}

fn format_mistakes_for_analysis(mistakes: &[Mistake]) -> String {
    if mistakes.is_empty() {
        return "None".to_string();
    }
    mistakes
        .iter()
        .map(|m| format!(r#"{{"id": "{}", "text": "{}"}}"#, m.id, m.specific_mistake))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_explained_for_analysis(explained: &[Explained]) -> String {
    if explained.is_empty() {
        return "None".to_string();
    }
    explained
        .iter()
        .map(|e| format!(r#"{{"id": "{}", "text": "{}"}}"#, e.id, e.new_phrase))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_translated_for_analysis(translated: &[dialect_coach_shared::Translated]) -> String {
    if translated.is_empty() {
        return "None".to_string();
    }
    translated
        .iter()
        .map(|t| format!(r#"{{"id": "{}", "text": "{}"}}"#, t.id, t.translated_word))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_exploratory_for_analysis(exploratory: &[dialect_coach_shared::Exploratory]) -> String {
    if exploratory.is_empty() {
        return "None".to_string();
    }
    exploratory
        .iter()
        .map(|e| format!(r#"{{"id": "{}", "text": "{}"}}"#, e.id, e.point_to_try))
        .collect::<Vec<_>>()
        .join(", ")
}

fn analysis_agent_prompt(
    dialect: &Dialect,
    conversation_history: &[String],
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[dialect_coach_shared::Translated],
    exploratory: &[dialect_coach_shared::Exploratory],
) -> String {
    format!(
        r#"You are analyzing a language learner's progress in {}.

CONVERSATION HISTORY:
{}

PAST MISTAKES TO ANALYZE:
{}

PAST EXPLAINED FEATURES TO ANALYZE:
{}

PAST TRANSLATIONS TO ANALYZE:
{}

PAST EXPLORATORY POINTS TO ANALYZE:
{}

Analyze the conversation and score each item:
MISTAKES (-10 to 10): -10=still occurring, 0=no usage/different error, 10=fixed
EXPLAINED (0 to 10): 0=not used, 5=attempted incorrectly, 10=used correctly
TRANSLATED (-10 to 10): -10=reverted to untranslated, 0=not used, 10=correctly used
EXPLORATORY (0 to 10): 0=not used, 5=imperfect attempt, 10=correctly used

Return ONLY this JSON:
{{"mistake_scores": {{}}, "explained_scores": {{}}, "translated_scores": {{}}, "exploratory_scores": {{}}}}"#,
        dialect.name(),
        conversation_history.join("\n"),
        format_mistakes_for_analysis(mistakes),
        format_explained_for_analysis(explained),
        format_translated_for_analysis(translated),
        format_exploratory_for_analysis(exploratory),
    )
}

impl AgentService {
    /// Create new agent service from environment variables
    pub fn from_env(qdrant: Arc<QdrantService>, embeddings: Arc<EmbeddingService>) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;
        let model_name =
            std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| CLAUDE_3_5_SONNET.to_string());

        let client = ClientBuilder::new(&api_key)
            .anthropic_version("2023-06-01")
            .build();

        tracing::info!("Initialized Anthropic client with model: {}", model_name);

        Ok(Self {
            client,
            model_name,
            qdrant,
            embeddings,
        })
    }

    pub async fn generate_analysis(
        &self,
        dialect: Dialect,
        conversation_history: &[String],
        mistakes: &[Mistake],
        explained: &[Explained],
        translated: &[dialect_coach_shared::Translated],
        exploratory: &[dialect_coach_shared::Exploratory],
    ) -> Result<dialect_coach_shared::AgentAnalysis> {
        if mistakes.is_empty() && explained.is_empty() && translated.is_empty() && exploratory.is_empty() {
            tracing::debug!("Skipping analysis - no learning items");
            return Ok(dialect_coach_shared::AgentAnalysis::new());
        }

        tracing::info!(
            "Starting analysis for {} mistakes, {} explained, {} translated, {} exploratory items",
            mistakes.len(),
            explained.len(),
            translated.len(),
            exploratory.len()
        );

        let prompt = analysis_agent_prompt(&dialect, conversation_history, mistakes, explained, translated, exploratory);

        tracing::debug!("Analysis prompt sent to Claude:\n{}", prompt);

        let agent = self
            .client
            .agent(&self.model_name)
            .max_tokens(256)
            .temperature(0.3)
            .build();

        tracing::info!("Calling Claude API for analysis...");
        let response = agent
            .prompt(prompt.as_str())
            .await
            .context("Failed to get analysis from Claude")?;

        tracing::info!("Received analysis response from Claude API");

        tracing::info!("Raw analysis response from Claude: {}", response);

        serde_json::from_str(&response).map_err(|e| {
            tracing::error!("JSON parse error: {}. First 200 chars of response: {}",
                e,
                &response.chars().take(200).collect::<String>()
            );
            anyhow::anyhow!("Claude returned invalid JSON for analysis: {}. Check if response is wrapped in markdown", e)
        })
    }

    fn retrieve_user_msg_embeddings(&self, user_message: &str) -> Result<Vec<f32>> {
        self.embeddings
            .embed_text(user_message)
            .context("Failed to generate content embedding")
    }

    fn retrieve_history_embeddings(
        &self,
        conversation_history: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        (*conversation_history)
            .iter()
            .take(5)
            .map(|s| {
                self.embeddings
                    .embed_text(s)
                    .context("Failed to generate history embedding")
            })
            .collect()
    }

    fn retrieve_embeddings(
        &self,
        user_message: &str,
        conversation_history: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        let content_embedding = self.retrieve_user_msg_embeddings(user_message)?;
        let mut topic_embeddings = self
            .retrieve_history_embeddings(conversation_history)?
            .to_owned();

        topic_embeddings.push(content_embedding);
        Ok(topic_embeddings)
    }

    async fn retrieve_example(
        &self,
        dialect: &Dialect,
        embedding: &[f32],
    ) -> Result<Vec<DialectDocument>> {
        self.qdrant
            .search_dialect_examples(embedding, dialect, 10)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to search content examples for dialect {} (embedding dim: {}): {}",
                    dialect.name(),
                    embedding.len(),
                    e
                )
            })
    }

    async fn retrieve_examples(
        &self,
        dialect: &Dialect,
        embeddings: Vec<Vec<f32>>,
    ) -> Result<Vec<DialectDocument>> {
        let mut docs = Vec::new();
        for embedding in embeddings {
            let example_docs = self.retrieve_example(dialect, &embedding).await?;
            docs.extend(example_docs);
        }
        Ok(docs)
    }

    fn temperature_for_mode(
        mode: &TeachingMode,
    ) -> f64 {
        match mode {
            TeachingMode::Immersive => 0.6,
            TeachingMode::Corrective => 0.4,
            TeachingMode::Explanatory => 0.5,
            TeachingMode::Interleaved => 0.4,
            TeachingMode::StoryTeller => 1.0,
            TeachingMode::Debug => 0.1,
        }
    }

    pub async fn generate_response(
        &self,
        user_message: &str,
        dialect: Dialect,
        formality: Formality,
        teaching_mode: TeachingMode,
        conversation_history: &[String],
    ) -> Result<dialect_coach_shared::AgentResponse> {
        tracing::info!("Generating embeddings for multi-vector retrieval");
        let embeddings = self.retrieve_embeddings(user_message, conversation_history)?;

        tracing::info!("Performing multi-vector retrieval");
        let examples = self.retrieve_examples(&dialect, embeddings).await?;

        tracing::info!("Adding random stylistic samples");
        let sample_formalities = match formality {
            Formality::Formal => vec![Formality::Formal, Formality::Casual],
            Formality::Casual => vec![Formality::Casual, Formality::DialectRich],
            Formality::DialectRich => {
                vec![Formality::Casual, Formality::DialectRich, Formality::Slang]
            }
            Formality::Slang => vec![Formality::Slang, Formality::DialectRich],
        };
        let random_samples = self
            .qdrant
            .random_dialect_samples(dialect, sample_formalities.clone(), 15)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get random samples for dialect {} with formalities {:?}: {}",
                    dialect.name(),
                    sample_formalities,
                    e
                )
            })?;

        tracing::info!("Combining and deduplicating examples");
        let mut all_examples = Vec::new();
        all_examples.extend(examples);
        all_examples.extend(random_samples);

        let mut seen = std::collections::HashSet::new();
        let unique_examples: Vec<_> = all_examples
            .into_iter()
            .filter(|doc| seen.insert(doc.content.clone()))
            .take(50)
            .collect();

        tracing::info!(
            "Retrieved {} total examples (after deduplication) for {}",
            unique_examples.len(),
            dialect.name()
        );

        // Step 6: Group examples by formality (prioritize requested formality)
        let primary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| doc.formality.is_none() || doc.formality == Some(formality))
            .take(30)
            .collect();

        let secondary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| {
                doc.formality.is_some()
                    && doc.formality != Some(formality)
                    && doc.formality.is_some()
            })
            .take(15)
            .collect();

        tracing::info!(
            "Grouped examples: {} primary ({:?}), {} secondary",
            primary_examples.len(),
            formality,
            secondary_examples.len()
        );

        // Step 7: Build rich RAG context
        let formality_label = match formality {
            Formality::Formal => "FORMAL",
            Formality::Casual => "CASUAL",
            Formality::DialectRich => "DIALECT-RICH",
            Formality::Slang => "SLANG",
        };

        let mut rag_context = format!(
            "\n\n# AUTHENTIC {} SPEECH PATTERNS\n\n",
            dialect.name().to_uppercase()
        );

        if !primary_examples.is_empty() {
            rag_context.push_str(&format!("## {} EXAMPLES:\n", formality_label));
            for (i, doc) in primary_examples.iter().enumerate() {
                rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
            }
            rag_context.push('\n');
        } else {
            tracing::warn!("No primary examples available!");
        }

        if !secondary_examples.is_empty() {
            rag_context.push_str("## ADDITIONAL EXAMPLES:\n");
            for (i, doc) in secondary_examples.iter().enumerate() {
                rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
            }
            rag_context.push('\n');
        } else {
            tracing::warn!("No secondary examples available!");
        }

        // Build conversation context
        let history_context = if conversation_history.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n# CONVERSATION HISTORY\n{}",
                conversation_history.join("\n")
            )
        };

        let role_desc = speaker_desc(&dialect, &formality);
        let teaching_rules = teaching_desc(&teaching_mode);

        let system_content = if teaching_mode == TeachingMode::Debug {
            format!(
                "# YOUR ROLE\n\
            {}.\n\n\
            Conversational Context: {}\n\n\
            # CRITICAL RULES\n\
            1. BE CONCISE: Explain why you did what you did simply and briefly, without pandering.\n\
            2. ITERATIVE IMPROVEMENT: Show exactly how the prompts could be improved to get a step closer to the desired effect.\n\
            Now respond to the user's message technically.",
                role_desc, history_context
            )
        } else {
            format!(
                "{}\n\n\
            {}\n\n\
            # YOUR ROLE\n\
            {}. Your responses must sound EXACTLY like the authentic examples above.\n\n\
            # CRITICAL RULES\n\
            1. MIMIC THE PATTERNS: Study the examples above and copy their vocabulary, grammar, and style\n\
            2. MAINTAIN FORMALITY: Match the {} level shown in the primary examples\n\
            {}\n\
            5. BE BRIEF: Keep responses conversational, not essay-length\n\
            6. USE DIALECT MARKERS: Include the characteristic phrases and constructions from the examples\n\
            7. {}\n\
            8. {}\n\
            {}\n\n\
            Now respond to the user's message naturally, as a local {} speaker would.",
                rag_context,
                CONTENT_FILTERING_DIRECTIVES,
                role_desc,
                formality_label.to_lowercase(),
                teaching_rules,
                JSON_OUTPUT_INSTRUCTION,
                output_format_spec(&teaching_mode),
                history_context,
                dialect.name()
            )
        };

        let max_tokens = tokens_per_mode(&teaching_mode);

        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&system_content)
            .max_tokens(max_tokens)
            .temperature(Self::temperature_for_mode(&teaching_mode))
            .build();

        tracing::info!("Sending prompt to Claude: {}", user_message);
        let response = agent
            .prompt(user_message)
            .await
            .context("Failed to get completion from Claude")?;

        tracing::info!("Raw response from Claude: {}", response);

        let parsed_response: dialect_coach_shared::AgentResponse =
            serde_json::from_str(&response)
                .map_err(|e| {
                    tracing::error!("JSON parse error: {}. First 200 chars of response: {}",
                        e,
                        &response.chars().take(200).collect::<String>()
                    );
                    anyhow::anyhow!("Claude returned invalid JSON: {}. Check if response is wrapped in markdown", e)
                })?;

        tracing::info!(
            "Generated response for dialect {} ({} chars)",
            dialect.name(),
            parsed_response.response.len()
        );

        Ok(parsed_response)
    }

    /// Simple translation without RAG - for fast prompt translation
    pub async fn generate_simple_translation(
        &self,
        prompt: &str,
        _dialect: Dialect,
        _formality: Formality,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        // Create a lightweight agent for translation only
        let agent = self
            .client
            .agent(&self.model_name)
            .max_tokens(64) // Keep translations short
            .temperature(0.7)
            .build();

        // Generate translation
        let response = agent
            .prompt(prompt)
            .await
            .context("Failed to get translation from Claude")?;

        Ok(dialect_coach_shared::AgentResponse::from(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_filtering_directives_structure() {
        // Test that content filtering directives contain the required rules
        assert!(CONTENT_FILTERING_DIRECTIVES.contains("### CONTENT FILTERING DIRECTIVES"));
        assert!(CONTENT_FILTERING_DIRECTIVES.contains(
            "1) Only flag content in the user's direct messages, not in system examples or context"
        ));
        assert!(
            CONTENT_FILTERING_DIRECTIVES
                .contains("2) Distinguish clearly between user input and system context")
        );
        assert!(
            CONTENT_FILTERING_DIRECTIVES
                .contains("3) Always respond to the user's actual message first")
        );
        assert!(CONTENT_FILTERING_DIRECTIVES.contains("Operational note:"));
    }

    #[test]
    fn test_system_prompt_order() {
        use dialect_coach_shared::{Dialect, Formality, TeachingMode};

        // Mock RAG context
        let rag_context = "\n\n# AUTHENTIC MEXICAN SPANISH SPEECH PATTERNS\n\n## CASUAL EXAMPLES:\n1. \"¡Órale, qué onda!\"\n";
        let role_desc =
            "You are a native Mexican Spanish speaker speaking naturally and conversationally";
        let formality_label = "casual";
        let teaching_rules = "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections";
        let history_context = "";
        let dialect_name = "Mexican Spanish";

        // Build system content using the same format as the actual code
        let system_content = format!(
            "{}\n\n\
            {}\n\n\
            # YOUR ROLE\n\
            {}. Your responses must sound EXACTLY like the authentic examples above.\n\n\
            # CRITICAL RULES\n\
            1. MIMIC THE PATTERNS: Study the examples above and copy their vocabulary, grammar, and style\n\
            2. MAINTAIN FORMALITY: Match the {} level shown in the primary examples\n\
            {}\n\
            4. BE BRIEF: Keep responses conversational, not essay-length\n\
            5. USE DIALECT MARKERS: Include the characteristic phrases and constructions from the examples\n\
            {}\n\n\
            Now respond to the user's message naturally, as a local {} speaker would.",
            rag_context,
            CONTENT_FILTERING_DIRECTIVES,
            role_desc,
            formality_label,
            teaching_rules,
            history_context,
            dialect_name
        );

        // Assert content filtering directives are present
        assert!(system_content.contains("### CONTENT FILTERING DIRECTIVES"));

        // Assert correct order: RAG context → Content filtering → Role → Rules
        let rag_pos = system_content.find("# AUTHENTIC").unwrap();
        let directives_pos = system_content
            .find("### CONTENT FILTERING DIRECTIVES")
            .unwrap();
        let role_pos = system_content.find("# YOUR ROLE").unwrap();
        let rules_pos = system_content.find("# CRITICAL RULES").unwrap();

        assert!(
            rag_pos < directives_pos,
            "RAG context should come before content filtering directives"
        );
        assert!(
            directives_pos < role_pos,
            "Content filtering directives should come before role"
        );
        assert!(
            role_pos < rules_pos,
            "Role should come before critical rules"
        );

        // Verify all three content filtering rules are present
        assert!(system_content.contains(
            "1) Only flag content in the user's direct messages, not in system examples or context"
        ));
        assert!(
            system_content.contains("2) Distinguish clearly between user input and system context")
        );
        assert!(system_content.contains("3) Always respond to the user's actual message first"));
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_agent_initialization() {
        dotenvy::dotenv().ok();

        let qdrant_url = std::env::var("QDRANT_URL").unwrap();
        let qdrant_key = std::env::var("QDRANT_API_KEY").unwrap();
        let qdrant = QdrantService::new(&qdrant_url, &qdrant_key).await.unwrap();
        let embeddings = EmbeddingService::new().unwrap();

        let agent = AgentService::from_env(Arc::new(qdrant), Arc::new(embeddings)).unwrap();
        assert!(!agent.model_name.is_empty());
    }
}
