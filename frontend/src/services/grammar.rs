use anyhow::{Context, Result};
use dialect_coach_shared::models::{Dialect, GrammarRequest, GrammarResponse, GrammarExplanation};
use gloo_net::http::Request;

#[derive(Clone)]
pub struct GrammarService {
    base_url: String,
}

impl GrammarService {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn explain_grammar(
        &self,
        phrase: &str,
        context: String,
        dialect: Dialect,
    ) -> Result<Vec<GrammarExplanation>> {
        let request_body = GrammarRequest {
            phrase: phrase.to_string(),
            context,
            dialect: dialect.id().to_string(),
        };

        let url = format!("{}/api/grammar", self.base_url);

        let response = Request::post(&url)
            .json(&request_body)?
            .send()
            .await
            .context("Failed to send grammar request")?;

        if !response.ok() {
            return Err(anyhow::anyhow!(
                "Grammar request failed with status: {}",
                response.status()
            ));
        }

        let grammar_response: GrammarResponse = response
            .json()
            .await
            .context("Failed to parse grammar response")?;

        if !grammar_response.success {
            return Err(anyhow::anyhow!(
                "Grammar explanation failed: {}",
                grammar_response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(grammar_response.explanations)
    }
}
