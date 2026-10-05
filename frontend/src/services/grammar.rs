use crate::services::connection::Connection;
use anyhow::Result;
use dialect_coach_shared::models::{Dialect, GrammarExplanation, GrammarRequest};
use dialect_coach_shared::{ClientMessage, Reply, StudyRequest};

#[derive(Clone)]
pub struct GrammarService {
    connection: Connection,
}

impl GrammarService {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub async fn explain_grammar(
        &self,
        phrase: &str,
        context: String,
        dialect: Dialect,
    ) -> Result<Vec<GrammarExplanation>> {
        let request = GrammarRequest {
            phrase: phrase.to_string(),
            context,
            dialect: dialect.id().to_string(),
        };
        let study = StudyRequest::Grammar(request);
        let reply = self.connection.request(ClientMessage::Study(study)).await?;
        explained(reply)
    }
}

fn explained(reply: Reply) -> Result<Vec<GrammarExplanation>> {
    match reply {
        Reply::Grammar(response) if response.success => Ok(response.explanations),
        Reply::Grammar(response) => Err(anyhow::anyhow!(
            "Grammar explanation failed: {}",
            response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
        )),
        other => unreachable!(
            "a grammar request is answered with Grammar, got {:?}",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::GrammarResponse;

    fn response(explanations: Vec<GrammarExplanation>, error: Option<&str>) -> Reply {
        Reply::Grammar(GrammarResponse {
            original_phrase: "hacés".to_string(),
            success: error.is_none(),
            explanations,
            error: error.map(str::to_string),
        })
    }

    #[test]
    fn test_explained_returns_explanations_and_names_failure() {
        let explanations = vec![GrammarExplanation {
            element: "voseo".to_string(),
            explanation: "vos takes hacés".to_string(),
        }];
        assert_eq!(
            explained(response(explanations.clone(), None)).unwrap(),
            explanations
        );
        let refused = explained(response(vec![], Some("Invalid dialect: x")));
        assert_eq!(
            refused.unwrap_err().to_string(),
            "Grammar explanation failed: Invalid dialect: x"
        );
    }
}
