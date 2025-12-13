use anyhow::{Context, Result};
use dialect_coach_shared::models::plan::import::SimpleImportLanguagePlan;
use dialect_coach_shared::models::Dialect;
use gloo_net::http::Request;
use web_sys::{File, FormData};

#[derive(PartialEq, Clone)]
pub struct PlanService {
    base_url: String,
}

impl PlanService {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn generate_plan(
        &self,
        text: Option<String>,
        file: Option<File>,
        dialect: Dialect,
    ) -> Result<SimpleImportLanguagePlan> {
        let url = format!("{}/api/plans/generate", self.base_url);
        let form_data = FormData::new().map_err(|e| anyhow::anyhow!("Failed to create FormData: {:?}", e))?;

        // 1. Add Dialect
        // We send the raw string value (e.g., "spanish_mexican") because the backend
        // manually wraps it in quotes before deserializing as a JSON string.
        let dialect_str = serde_json::to_string(&dialect)?.trim_matches('"').to_string();
        form_data.append_with_str("dialect", &dialect_str)
            .map_err(|e| anyhow::anyhow!("Failed to append dialect: {:?}", e))?;

        // 2. Add Content (Text or File)
        if let Some(t) = text {
            form_data.append_with_str("text", &t)
                .map_err(|e| anyhow::anyhow!("Failed to append text: {:?}", e))?;
        } else if let Some(f) = file {
            // "file" or "text" logic in backend handles either under those names
            form_data.append_with_blob("file", &f)
                .map_err(|e| anyhow::anyhow!("Failed to append file: {:?}", e))?;
        } else {
            return Err(anyhow::anyhow!("Must provide either text or file"));
        }

        // 3. Send Request
        // gloo_net automatically sets Content-Type to multipart/form-data when body is FormData
        let response = Request::post(&url)
            .body(form_data)?
            .send()
            .await
            .context("Failed to connect to server")?;

        if !response.ok() {
            let error_msg = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
             return Err(anyhow::anyhow!("Server error {}: {}", response.status(), error_msg));
        }

        let plan: SimpleImportLanguagePlan = response
            .json()
            .await
            .context("Invalid response from server")?;

        Ok(plan)
    }
}
