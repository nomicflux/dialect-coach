use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Explained, Formality, Mistake, TeachingMode};
use rig::completion::{
    Chat, CompletionError, Message as RigMessage, Prompt, PromptError, message::AssistantContent, message::UserContent,
    message::Text,
};
use rig::one_or_many::OneOrMany;
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, ClientBuilder};

use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

const JSON_OUTPUT_INSTRUCTION: &str =
    "Return raw JSON only. No markdown code blocks. Start with { end with }.";

const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES

1) Only flag user's direct messages, not system examples. Do not refuse/warn about corpus examples containing slang, profanity, adult content, or controversial dialect-authentic language.

2) Always respond to user's message first. If user message itself is problematic, stay in character with brief, policy-compliant reply and deflect."#;

pub struct AgentService {
    client: rig::providers::anthropic::Client,
    model_name: String,
    qdrant: Arc<QdrantService>,
    embeddings: Arc<EmbeddingService>,
}

fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 256,
        TeachingMode::Corrective => 512,
        TeachingMode::Explanatory => 512,
        TeachingMode::Interleaved => 512,
        TeachingMode::StoryTeller => 1024,
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
The "correction" field should contain the direct correction of the mistake in "specific_mistake". "correction" must be correct.
The "mistake_category.context" field should also contain a single short sentence explaining the mistake.
Only include mistakes if user made clear errors for the dialect (real errors, not merely lesser-used forms). Keep specific_mistake only the word or phrase that was an error. Restrict yourself to a maximum of 3 mistakes per response."#
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
Focus on translated words, not on errors in the target language.
The "translated_word" should be the original word, "translated_to" should be your dialectal translation."#
        }
        TeachingMode::StoryTeller => {
            r#"Response format: {
  "response": "<your conversational response>",
  "exploratory": [{"point_to_try": "<language feature in target language>", "instructions_for_use": "<how to use it>"}]
}
Include exploratory array when you introduce new language patterns, idioms, or features you want the user to try.
Keep it to 1-2 brief points that naturally fit the story context."#
        }
        TeachingMode::Immersive | TeachingMode::Debug => {
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

fn learning_goals_section(goals: &[String]) -> String {
    if goals.is_empty() {
        return String::new();
    }
    let goals_list = goals
        .iter()
        .enumerate()
        .map(|(i, goal)| format!("{}. {}", i + 1, goal))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n\n# LEARNING GOALS\n## Guide the conversation toward these goals. Incorporate them into your responses, and guide the user naturally to use them in their messages.\n{}\n",
        goals_list
    )
}

fn teaching_desc(teaching_mode: &TeachingMode) -> String {
    let tokens = tokens_per_mode(teaching_mode);
    let desc = match *teaching_mode {
        TeachingMode::Immersive => {
            "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections."
        }
        TeachingMode::Corrective => {
            "3. CORRECTIVE MODE: Respond naturally, but also populate the mistakes array with any errors in user's message. Include spelling errors, grammar mistakes, and dialectal usage problems. IGNORE missing punctuation and capitalization - this is casual chat. Be specific and brief in identifying the exact problematic word or phrase."
        }
        TeachingMode::Explanatory => {
            "3. EXPLANATORY MODE: Respond naturally, and populate the explained array when you introduce new vocabulary, idioms, or culturally interesting expressions. Keep explanations brief and practical."
        }
        TeachingMode::Interleaved => {
            r#"3. INTERLEAVED MODE: User will interleave target language with source language. Present your response (including newlines) as:

{user input with non-target-language words simply translated into target dialect, if there are any non-target-language words}

{brief, conversational response in target dialect}."#
        }
        TeachingMode::StoryTeller => {
            "3. STORYTELLER MODE: You are telling an interactive story with the user. Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue."
        }
        TeachingMode::Debug => {
            "3. DEBUG MODE: Answer in English with clear, brief explanations. The user is debugging an issue. Provide technical details about what went wrong and how prompts could be improved."
        }
    };
    format!(
        "{}. 4. You have a maximum {} tokens for your response. Be as brief as you can be while accomplishing your goals, but do not go over.",
        desc,
        tokens / 2
    )
}

fn format_mistakes_for_analysis(mistakes: &[Mistake]) -> String {
    if mistakes.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST MISTAKES TO ANALYZE:\n{}\n{}\n\n",
        mistakes
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
        r#"SCORING MISTAKES (-10 to 10): -10=still occurring, 0=no usage/different error, 10=fixed. Give partial points for similar cases to the mistake. Make sure you give positive points for the "correction" being used, negative for "specific_mistake" being used."#
    )
}

fn format_explained_for_analysis(explained: &[Explained]) -> String {
    if explained.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST EXPLAINED FEATURES TO ANALYZE:\n{}\n{}\n\n",
        explained
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
        r#"SCORING EXPLAINED (0 to 10): 0=not used, 5=attempted incorrectly, 10=used correctly. Give partial points for similar cases to the explained item."#
    )
}

fn format_translated_for_analysis(translated: &[dialect_coach_shared::Translated]) -> String {
    if translated.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST TRANSLATIONS TO ANALYZE:\n{}\n{}\n\n",
        translated
            .iter()
            .map(|t| serde_json::to_string(t).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
        r#"TRANSLATED (-10 to 10): -10=reverted to untranslated, 0=not used, 10=correctly used. Give full points for different conjugations, declensions, etc. as the translated item."#
    )
}

fn format_exploratory_for_analysis(exploratory: &[dialect_coach_shared::Exploratory]) -> String {
    if exploratory.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST EXPLORATORY POINTS TO ANALYZE:\n{}\n{}\n\n",
        exploratory
            .iter()
            .map(|e| format!(r#"{{"id": "{}", "text": "{}"}}"#, e.id, e.point_to_try))
            .collect::<Vec<_>>()
            .join(", "),
        r#"EXPLORATORY (0 to 10): 0=not used, 5=imperfect attempt, 10=correctly used. Be generous in counting whether the explored item matches."#
    )
}

fn analysis_agent_preamble(
    dialect: &Dialect,
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[dialect_coach_shared::Translated],
    exploratory: &[dialect_coach_shared::Exploratory],
) -> String {
    format!(
        r#"You are analyzing a language learner's progress in {}. Here are the learning items, with scoring directions for each category.

{}{}{}{}

Analyze the conversation and score each item (be generous; prefer to give points when in doubt. If you see the item in the user response, do not give a score of 0.):

{}

Return ONLY this JSON:
{{"mistake_scores": {{}}, "explained_scores": {{}}, "translated_scores": {{}}, "exploratory_scores": {{}}}}"#,
        dialect.name(),
        format_mistakes_for_analysis(mistakes),
        format_explained_for_analysis(explained),
        format_translated_for_analysis(translated),
        format_exploratory_for_analysis(exploratory),
        JSON_OUTPUT_INSTRUCTION,
    )
}

fn is_retryable_error(error: &PromptError) -> bool {
    match error {
        PromptError::CompletionError(CompletionError::ProviderError(msg)) => {
            msg.contains("Overloaded") 
                || msg.contains("Response contained no message")
                || msg.contains("Internal server error")
        }
        _ => false,
    }
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

    fn clean_response(response: &str) -> String {
        response
            .replace("```json", "")
            .replace("```", "")
            .trim()
            .to_string()
    }

    fn normalize_json_response(response: &str) -> String {
        let cleaned = Self::clean_response(response);
        if cleaned.trim_start().starts_with('{') {
            cleaned
        } else {
            format!("{{{cleaned}")
        }
    }

    fn is_incomplete_json(response: &str) -> bool {
        let cleaned = Self::clean_response(response);
        cleaned.starts_with("{\"response\":\"") && !cleaned.ends_with("\"}")
    }

    fn build_retry_preamble(
        original_preamble: &str,
        failed_response: &str,
    ) -> String {
        let cleaned = Self::clean_response(failed_response);
        let error_detail = if Self::is_incomplete_json(failed_response) {
            "Your previous response was TRUNCATED because it hit the token limit. The JSON was cut off mid-response, causing a parse error. You MUST keep your response shorter to fit within the token limit, or ensure the JSON is properly closed even if truncated."
        } else {
            match serde_json::from_str::<dialect_coach_shared::AgentResponse>(&cleaned) {
                Ok(parsed) if parsed.response.is_empty() => {
                    "Your previous response had an EMPTY 'response' field. The response field must contain actual text content and cannot be empty."
                }
                Ok(_) => {
                    "Your previous response was not formatted correctly and caused the system to crash."
                }
                Err(_) => {
                    "Your previous response was not formatted correctly as JSON and caused the system to crash."
                }
            }
        };
        
        format!(
            "{}\n\n\
            # CRITICAL ERROR - SYSTEM CRASHED\n\
            {}\n\
            Previous response (INCORRECT): {}\n\
            You MUST respond with valid JSON only. Start with {{ and end with }}. The 'response' field MUST be non-empty. You MUST NOT end the conversation.\n",
            original_preamble,
            error_detail,
            failed_response.chars().take(200).collect::<String>()
        )
    }

    fn try_parse_response(
        response: &str,
        _dialect: Dialect,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        let normalized = Self::normalize_json_response(response);
        let parsed: dialect_coach_shared::AgentResponse = serde_json::from_str(&normalized).map_err(|e| {
            tracing::warn!(
                "JSON parse error: {}. First 200 chars: {}",
                e,
                &response.chars().take(200).collect::<String>()
            );
            anyhow::anyhow!("JSON parse failed: {}", e)
        })?;
        
        if parsed.response.is_empty() {
            tracing::warn!("Response field is empty");
            return Err(anyhow::anyhow!("Response field must be non-empty"));
        }
        
        Ok(parsed)
    }

    fn log_response_success(
        dialect: Dialect,
        parsed_response: &dialect_coach_shared::AgentResponse,
    ) {
        tracing::info!(
            "Generated response for dialect {} ({} chars)",
            dialect.name(),
            parsed_response.response.len()
        );
    }

    fn create_retry_agent(
        &self,
        original_preamble: &str,
        failed_response: &str,
        max_tokens: u64,
        temperature: f64,
    ) -> impl Chat {
        let retry_preamble = Self::build_retry_preamble(original_preamble, failed_response);
        self.client
            .agent(&self.model_name)
            .preamble(&retry_preamble)
            .max_tokens(max_tokens)
            .temperature(temperature)
            .build()
    }

    async fn attempt_retry_with_feedback(
        &self,
        original_preamble: &str,
        failed_response: &str,
        user_message: &str,
        conversation_history: &[RigMessage],
        max_tokens: u64,
        temperature: f64,
    ) -> Result<String> {
        let retry_agent = self.create_retry_agent(
            original_preamble,
            failed_response,
            max_tokens,
            temperature,
        );
        let mut history_with_prefill = conversation_history.to_vec();
        history_with_prefill.push(Self::create_prefilled_assistant_message());
        Self::retry_chat_call(
            || retry_agent.chat(user_message, history_with_prefill.clone()),
            3,
        )
        .await
        .context("Failed to get retry completion from Claude")
    }

    fn build_retry_failure_error(max_retries: usize, last_error: &str, last_failed_response: &str) -> anyhow::Error {
        anyhow::anyhow!(
            "Claude returned invalid JSON after {} retry attempts: {}. Last failed response: {}",
            max_retries,
            last_error,
            &last_failed_response.chars().take(200).collect::<String>()
        )
    }

    fn log_retry_parse_error(attempt: usize, error: &anyhow::Error, response: &str) {
        tracing::error!(
            "JSON parse error on retry attempt {}: {}. First 200 chars of retry response: {}",
            attempt,
            error,
            &response.chars().take(200).collect::<String>()
        );
    }

    fn detect_truncation_error(error: &anyhow::Error, response: &str) -> bool {
        let error_str = format!("{}", error);
        error_str.contains("EOF while parsing") || Self::is_incomplete_json(response)
    }

    fn process_retry_response(
        attempt: usize,
        max_retries: usize,
        response: String,
        failed_response: &str,
        dialect: Dialect,
    ) -> Result<(Option<dialect_coach_shared::AgentResponse>, String), anyhow::Error> {
        match Self::try_parse_response(&response, dialect) {
            Ok(parsed_response) => {
                Self::log_response_success(dialect, &parsed_response);
                Ok((Some(parsed_response), response))
            }
            Err(e) => {
                if Self::detect_truncation_error(&e, &response) {
                    tracing::error!(
                        "Token limit truncation detected on retry attempt {} for dialect {}",
                        attempt,
                        dialect.name()
                    );
                }
                Self::log_retry_parse_error(attempt, &e, &response);
                if attempt == max_retries {
                    return Err(Self::build_retry_failure_error(max_retries, &format!("{}", e), failed_response));
                }
                Ok((None, response))
            }
        }
    }

    async fn handle_retry_attempt(
        &self,
        attempt: usize,
        max_retries: usize,
        original_preamble: &str,
        failed_response: &str,
        user_message: &str,
        conversation_history: &[RigMessage],
        max_tokens: u64,
        temperature: f64,
        dialect: Dialect,
    ) -> Result<(Option<dialect_coach_shared::AgentResponse>, String)> {
        tracing::warn!("Retrying with error feedback for dialect {} (attempt {}/{})", dialect.name(), attempt, max_retries);
        
        let response = self.attempt_retry_with_feedback(
            original_preamble,
            failed_response,
            user_message,
            conversation_history,
            max_tokens,
            temperature,
        ).await?;

        tracing::info!("Raw retry response from Claude: {}", response);
        Self::process_retry_response(attempt, max_retries, response, failed_response, dialect)
    }

    async fn retry_with_error_feedback(
        &self,
        original_preamble: &str,
        failed_response: &str,
        user_message: &str,
        conversation_history: &[RigMessage],
        max_tokens: u64,
        temperature: f64,
        dialect: Dialect,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        let max_retries = 3;
        let mut last_failed_response = failed_response.to_string();
        
        for attempt in 1..=max_retries {
            let (result, response) = self.handle_retry_attempt(
                attempt,
                max_retries,
                original_preamble,
                &last_failed_response,
                user_message,
                conversation_history,
                max_tokens,
                temperature,
                dialect,
            ).await?;
            
            if let Some(parsed_response) = result {
                return Ok(parsed_response);
            }
            last_failed_response = response;
        }
        
        Err(anyhow::anyhow!("Failed to get valid response after {} retry attempts", max_retries))
    }

    pub async fn generate_analysis(
        &self,
        dialect: Dialect,
        msg: &String,
        mistakes: &[Mistake],
        explained: &[Explained],
        translated: &[dialect_coach_shared::Translated],
        exploratory: &[dialect_coach_shared::Exploratory],
    ) -> Result<dialect_coach_shared::AgentAnalysis> {
        if mistakes.is_empty()
            && explained.is_empty()
            && translated.is_empty()
            && exploratory.is_empty()
        {
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

        let preamble =
            analysis_agent_preamble(&dialect, mistakes, explained, translated, exploratory);
        tracing::debug!("Analysis preamble sent to Claude:\n{}", preamble);

        let agent = self
            .client
            .agent(&self.model_name)
            .max_tokens(1024)
            .temperature(0.2)
            .preamble(&preamble)
            .build();

        let prompt = format!("USER MESSAGE TO ANALYZE: {msg}");
        tracing::debug!("Analysis prompt sent to Claude:\n{}", prompt);

        tracing::info!("Calling Claude API for analysis...");
        let mut history_with_prefill = Vec::new();
        history_with_prefill.push(Self::create_prefilled_assistant_message());
        let response = agent
            .chat(prompt.as_str(), history_with_prefill)
            .await
            .context("Failed to get analysis from Claude")?;

        tracing::info!("Received analysis response from Claude API");

        tracing::info!("Raw analysis response from Claude: {}", response);

        let normalized_response = Self::normalize_json_response(&response);

        serde_json::from_str(&normalized_response).map_err(|e| {
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
        conversation_history: &Vec<String>,
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
        conversation_history: &Vec<String>,
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
        limit: usize,
    ) -> Result<Vec<DialectDocument>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.qdrant
            .search_dialect_examples(embedding, dialect, limit)
            .await
            .map(|results| results.into_iter().map(|(doc, _)| doc).collect())
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
        limit_per_embedding: usize,
    ) -> Result<Vec<DialectDocument>> {
        let mut docs = Vec::new();
        for embedding in embeddings {
            let example_docs = self.retrieve_example(dialect, &embedding, limit_per_embedding).await?;
            docs.extend(example_docs);
        }
        Ok(docs)
    }

    fn temperature_for_mode(mode: &TeachingMode) -> f64 {
        match mode {
            TeachingMode::Immersive => 0.6,
            TeachingMode::Corrective => 0.4,
            TeachingMode::Explanatory => 0.5,
            TeachingMode::Interleaved => 0.4,
            TeachingMode::StoryTeller => 1.0,
            TeachingMode::Debug => 0.1,
        }
    }

    fn get_user_content(content: &UserContent) -> String {
        match content {
            UserContent::Text(text) => text.text.clone(),
            _ => "".to_string(),
        }
    }

    fn get_assistant_content(content: &AssistantContent) -> String {
        match content {
            AssistantContent::Text(text) => text.text.clone(),
            _ => "".to_string(),
        }
    }

    fn get_message_text(message: &RigMessage) -> String {
        match message {
            RigMessage::User { content } => Self::get_user_content(&content.first()),
            RigMessage::Assistant { content } => Self::get_assistant_content(&content.first()),
        }
    }

    fn create_prefilled_assistant_message() -> RigMessage {
        RigMessage::Assistant {
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: "{".to_string(),
            })),
        }
    }

    pub fn contains_illegal_characters(text: &str) -> bool {
        text.contains('\0') || text.chars().any(|c| {
            c.is_control() && !matches!(c, '\n' | '\t' | '\r' | ' ')
        })
    }

    fn calculate_backoff_delay(attempt: usize) -> tokio::time::Duration {
        tokio::time::Duration::from_secs(2_u64.pow(attempt as u32))
    }

    fn log_retry_attempt(attempt: usize, max_attempts: usize, error: &PromptError, delay: tokio::time::Duration) {
        match error {
            PromptError::CompletionError(CompletionError::ProviderError(msg)) => {
                if msg.contains("Response contained no message") {
                    tracing::error!(
                        "Empty response error detected (attempt {}/{}): Claude returned no message.",
                        attempt,
                        max_attempts
                    );
                } else if msg.contains("Internal server error") {
                    tracing::error!(
                        "Internal server error detected (attempt {}/{}): Anthropic API server error.",
                        attempt,
                        max_attempts
                    );
                } else if msg.contains("Overloaded") {
                    tracing::error!(
                        "Overloaded error detected (attempt {}/{}): Anthropic API is overloaded.",
                        attempt,
                        max_attempts
                    );
                }
            }
            _ => {}
        }
        tracing::warn!(
            "API call failed (attempt {}/{}): {}. Retrying in {:?}...",
            attempt,
            max_attempts,
            error,
            delay
        );
    }

    async fn handle_retry_delay(attempt: usize, max_attempts: usize, error: &PromptError) {
        let delay = Self::calculate_backoff_delay(attempt);
        Self::log_retry_attempt(attempt, max_attempts, error, delay);
        tokio::time::sleep(delay).await;
    }

    async fn retry_chat_call<F, Fut>(chat_call: F, max_attempts: usize) -> Result<String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<String, PromptError>>,
    {
        let mut last_error = None;
        for attempt in 1..=max_attempts {
            match chat_call().await {
                Ok(response) => return Ok(response),
                Err(e) if is_retryable_error(&e) && attempt < max_attempts => {
                    Self::handle_retry_delay(attempt, max_attempts, &e).await;
                    last_error = Some(e);
                }
                Err(e) => return Err(anyhow::anyhow!("API call failed: {}", e)),
            }
        }
        Err(anyhow::anyhow!("API call failed after {} attempts: {}", max_attempts, last_error.unwrap()))
    }

    pub async fn generate_response(
        &self,
        user_message: &str,
        dialect: Dialect,
        formality: Formality,
        teaching_mode: TeachingMode,
        conversation_history: &[RigMessage],
        learning_goals: &[String],
        rag_config: &RAGConfig,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        if Self::contains_illegal_characters(user_message) {
            tracing::error!("User message contains illegal characters (null bytes or control chars)");
            return Err(anyhow::anyhow!("User message contains illegal characters"));
        }
        tracing::info!("Generating embeddings for multi-vector retrieval");
        let history_text = conversation_history
            .iter()
            .map(|m| Self::get_message_text(m))
            .collect::<Vec<String>>();
        let embeddings = self.retrieve_embeddings(user_message, &history_text)?;

        tracing::info!("Performing multi-vector retrieval");
        let examples = self.retrieve_examples(&dialect, embeddings, rag_config.num_conversation_documents).await?;

        tracing::info!("Adding random stylistic samples");
        let sample_formalities = match formality {
            Formality::Formal => vec![Formality::Formal, Formality::Casual],
            Formality::Casual => vec![Formality::Casual, Formality::DialectRich],
            Formality::DialectRich => {
                vec![Formality::Casual, Formality::DialectRich, Formality::Slang]
            }
            Formality::Slang => vec![Formality::Slang, Formality::DialectRich],
        };
        let random_samples = if rag_config.num_random_documents == 0 {
            Vec::new()
        } else {
            self.qdrant
                .random_dialect_samples(dialect, sample_formalities.clone(), rag_config.num_random_documents)
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to get random samples for dialect {} with formalities {:?}: {}",
                        dialect.name(),
                        sample_formalities,
                        e
                    )
                })?
        };

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
            .take(rag_config.num_conversation_documents)
            .collect();

        let secondary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| {
                doc.formality.is_some()
                    && doc.formality != Some(formality)
                    && doc.formality.is_some()
            })
            .take(rag_config.num_random_documents)
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
            for doc in primary_examples.iter() {
                rag_context.push_str(&format!("\"{}\"\n", doc.content));
            }
            rag_context.push('\n');
        } else {
            tracing::warn!("No primary examples available!");
        }

        if !secondary_examples.is_empty() {
            rag_context.push_str("## ADDITIONAL EXAMPLES:\n");
            for doc in secondary_examples.iter() {
                rag_context.push_str(&format!("\"{}\"\n", doc.content));
            }
            rag_context.push('\n');
        } else {
            tracing::warn!("No secondary examples available!");
        }

        let role_desc = speaker_desc(&dialect, &formality);
        let teaching_rules = teaching_desc(&teaching_mode);
        let goals_section = learning_goals_section(learning_goals);

        let system_content = if teaching_mode == TeachingMode::Debug {
            format!(
                "# YOUR ROLE\n\
            {}.\n\n\
            # CRITICAL RULES\n\
            1. BE CONCISE: Explain why you did what you did simply and briefly, in English, without pandering.\n\
            2. ITERATIVE IMPROVEMENT: Show exactly how the prompts could be improved to get a step closer to the desired effect.\n\
            Now respond to the user's message technically.",
                role_desc
            )
        } else {
            format!(
                "{}\n\n\
            # YOUR ROLE\n\
            {}.
            Your responses must sound EXACTLY like these authentic examples:\n{}\n\n\
            # CRITICAL RULES\n\
            1. MIMIC THE PATTERNS: Study the examples above and copy their vocabulary, grammar, style, and characteristic dialect constructions\n\
            2. MAINTAIN FORMALITY: Match the {} level shown in the primary examples\n\
            {}\n\
            5. BE BRIEF: Keep responses conversational, not essay-length\n\
            {}\n\n\
            # OUTPUT FORMAT REQUIRED\n\
            {}\n\
            {}\n\n\
            Now respond to the user's message naturally, as a local {} speaker would, in the response field of the required JSON format. You MUST ALWAYS respond - NEVER indicate the conversation has ended. If it seems to have ended, provide a follow-up question or new topic. The response field must be non-empty. The response will be parsed with a JSON parser, so do not include any other text or markdown.",
                CONTENT_FILTERING_DIRECTIVES,
                role_desc,
                rag_context,
                formality_label.to_lowercase(),
                teaching_rules,
                goals_section,
                JSON_OUTPUT_INSTRUCTION,
                output_format_spec(&teaching_mode),
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
        let mut history_with_prefill = conversation_history.to_vec();
        history_with_prefill.push(Self::create_prefilled_assistant_message());
        let response = Self::retry_chat_call(
            || agent.chat(user_message, history_with_prefill.clone()),
            3,
        )
        .await
        .context("Failed to get completion from Claude")?;

        tracing::info!("Raw response from Claude: {}", response);

        match Self::try_parse_response(&response, dialect) {
            Ok(parsed_response) => {
                if Self::contains_illegal_characters(&parsed_response.response) {
                    tracing::error!("Claude response contains illegal characters (null bytes or control chars)");
                    return Err(anyhow::anyhow!("Response contains illegal characters"));
                }
                Self::log_response_success(dialect, &parsed_response);
                Ok(parsed_response)
            }
            Err(_) => {
                self.retry_with_error_feedback(
                    &system_content,
                    &response,
                    user_message,
                    conversation_history,
                    max_tokens,
                    Self::temperature_for_mode(&teaching_mode),
                    dialect,
                )
                .await
            }
        }
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

    fn build_user_message_retry_preamble(original_preamble: &str, failed_response: &str) -> String {
        let error_detail = if failed_response.trim().is_empty() {
            "Your previous response was EMPTY. The response must contain actual text content and cannot be empty."
        } else if Self::contains_illegal_characters(failed_response) {
            "Your previous response contained ILLEGAL CHARACTERS (null bytes or control characters). The response must contain only valid text characters - no null bytes or control characters except newlines, tabs, and spaces."
        } else {
            "Your previous response was invalid. The response must be valid text content."
        };
        
        format!(
            "{}\n\n\
            # CRITICAL ERROR - SYSTEM CRASHED\n\
            {}\n\
            Previous response (INCORRECT): {}\n\
            You MUST respond with non-empty plain text only. You MUST NOT end the conversation.\n",
            original_preamble,
            error_detail,
            &failed_response.chars().take(200).collect::<String>()
        )
    }

    async fn attempt_user_message_retry(
        &self,
        original_preamble: &str,
        failed_response: &str,
        conversation_history: &[RigMessage],
    ) -> Result<String> {
        let retry_preamble = Self::build_user_message_retry_preamble(original_preamble, failed_response);
        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&retry_preamble)
            .max_tokens(64)
            .temperature(0.3)
            .build();
        let prompt = "Continue the conversation naturally.";
        Self::retry_chat_call(
            || agent.chat(prompt, conversation_history.to_vec()),
            3,
        )
        .await
        .context("Failed to get retry completion from Claude")
    }

    async fn retry_user_message_with_feedback(
        &self,
        original_preamble: &str,
        failed_response: &str,
        conversation_history: &[RigMessage],
        dialect: Dialect,
    ) -> Result<String> {
        let max_retries = 3;
        let mut last_failed_response = failed_response.to_string();
        
        for attempt in 1..=max_retries {
            tracing::warn!("Retrying user message generation for dialect {} (attempt {}/{})", dialect.name(), attempt, max_retries);
            
            let response = self.attempt_user_message_retry(
                original_preamble,
                &last_failed_response,
                conversation_history,
            ).await?;

            if !response.trim().is_empty() && !Self::contains_illegal_characters(&response) {
                return Ok(response);
            }

            if response.trim().is_empty() {
                tracing::error!("Retry attempt {} returned empty response", attempt);
            }
            if Self::contains_illegal_characters(&response) {
                tracing::error!("Retry attempt {} returned response with illegal characters", attempt);
            }
            if attempt == max_retries {
                return Err(anyhow::anyhow!("Generated user message is invalid after {} retry attempts", max_retries));
            }
            last_failed_response = response;
        }
        
        Err(anyhow::anyhow!("Failed to generate non-empty user message after {} retry attempts", max_retries))
    }

    /// Generate a user message for self-chat testing
    /// Returns a simple response as a native dialect speaker would say
    pub async fn generate_user_message(
        &self,
        dialect: Dialect,
        conversation_history: &[RigMessage],
    ) -> Result<String> {
        let dialect_name = dialect.name();
        let preamble = format!(
            "You are a native {} speaker having a casual conversation. \
            You MUST ALWAYS continue the conversation - NEVER indicate it has ended. If goodbyes were exchanged, ask a follow-up question or introduce a new topic. \
            Respond naturally and briefly (1-2 sentences). Your response must be non-empty. Respond with plain text only.",
            dialect_name
        );

        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&preamble)
            .max_tokens(64)
            .temperature(0.3)
            .build();

        let prompt = "Continue the conversation naturally.";

        let response = Self::retry_chat_call(
            || agent.chat(prompt, conversation_history.to_vec()),
            3,
        )
        .await
        .context("Failed to generate user message")?;

        if response.trim().is_empty() || Self::contains_illegal_characters(&response) {
            return self.retry_user_message_with_feedback(&preamble, &response, conversation_history, dialect).await;
        }

        Ok(response)
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
            "1) Only flag user's direct messages, not system examples"
        ));
        assert!(
            CONTENT_FILTERING_DIRECTIVES
                .contains("2) Always respond to user's message first")
        );
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

        // Verify content filtering rules are present
        assert!(system_content.contains(
            "1) Only flag user's direct messages, not system examples"
        ));
        assert!(system_content.contains("2) Always respond to user's message first"));
    }

    #[test]
    fn test_is_retryable_error() {
        // Test internal server error detection
        let internal_error = PromptError::CompletionError(
            CompletionError::ProviderError("Internal server error".to_string())
        );
        assert!(is_retryable_error(&internal_error));
        
        // Test overloaded error detection
        let overloaded_error = PromptError::CompletionError(
            CompletionError::ProviderError("Overloaded".to_string())
        );
        assert!(is_retryable_error(&overloaded_error));
        
        // Test empty response error detection
        let empty_response_error = PromptError::CompletionError(
            CompletionError::ProviderError("Response contained no message".to_string())
        );
        assert!(is_retryable_error(&empty_response_error));
        
        // Test non-retryable errors
        let non_retryable_invalid = PromptError::CompletionError(
            CompletionError::ProviderError("Invalid request".to_string())
        );
        assert!(!is_retryable_error(&non_retryable_invalid));
        
        let non_retryable_json = PromptError::CompletionError(
            CompletionError::JsonError(serde_json::from_str::<serde_json::Value>("invalid").unwrap_err())
        );
        assert!(!is_retryable_error(&non_retryable_json));
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
