use crate::services::connection::Connection;
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dialect_coach_shared::models::Dialect;
use dialect_coach_shared::models::plan::import::SimpleImportLanguagePlan;
use dialect_coach_shared::{ClientMessage, PlanRequest, PlanSource, Reply, StudyRequest};
use wasm_bindgen_futures::JsFuture;
use web_sys::File;

#[derive(PartialEq, Clone)]
pub struct PlanService {
    connection: Connection,
}

impl PlanService {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub async fn generate_plan(
        &self,
        text: Option<String>,
        file: Option<File>,
        dialect: Dialect,
    ) -> Result<SimpleImportLanguagePlan> {
        let source = match (text, file) {
            (Some(text), _) => PlanSource::Text(text),
            (None, Some(file)) => file_source(&file).await?,
            (None, None) => return Err(anyhow::anyhow!("Must provide either text or file")),
        };
        let study = StudyRequest::GeneratePlan(PlanRequest { dialect, source });
        let reply = self.connection.request(ClientMessage::Study(study)).await?;
        planned(reply)
    }
}

/// The file's type and its bytes, base64-encoded for the connection.
async fn file_source(file: &File) -> Result<PlanSource> {
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read file: {:?}", e))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    Ok(PlanSource::File {
        content_type: file.type_(),
        base64: STANDARD.encode(bytes),
    })
}

fn planned(reply: Reply) -> Result<SimpleImportLanguagePlan> {
    match reply {
        Reply::Plan(result) => result.map_err(|e| anyhow::anyhow!("Server error: {}", e)),
        other => unreachable!("a plan request is answered with Plan, got {:?}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planned_returns_plan_and_names_failure() {
        let plan = SimpleImportLanguagePlan {
            title: "Greetings".to_string(),
            dialect: Dialect::SpanishArgentinian,
            description: None,
            steps: vec![],
        };
        assert_eq!(planned(Reply::Plan(Ok(plan))).unwrap().title, "Greetings");
        let refused = planned(Reply::Plan(Err("Empty text content".to_string())));
        assert_eq!(
            refused.unwrap_err().to_string(),
            "Server error: Empty text content"
        );
    }
}
