use anyhow::{Result, anyhow, Context};
use crate::agent_service::provider::{ProviderAgentConfig, ANTHROPIC_PROVIDER, OPENAI_PROVIDER};
use serde_json::json;
use std::time::Duration;

pub struct OcrService;

use base64::prelude::*;

impl OcrService {
    pub async fn transcribe_images(
        images: &[Vec<u8>],
        config: &ProviderAgentConfig,
    ) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        match config.provider.as_str() {
            ANTHROPIC_PROVIDER => Self::transcribe_anthropic(&client, images, config).await,
            OPENAI_PROVIDER => Self::transcribe_openai(&client, images, config).await,
            _ => Err(anyhow!("Unsupported provider for Vision OCR: {}", config.provider)),
        }
    }

    async fn transcribe_anthropic(
        client: &reqwest::Client,
        images: &[Vec<u8>],
        config: &ProviderAgentConfig,
    ) -> Result<String> {
        let mut content_blocks = Vec::new();

        // Add plain text prompt
        content_blocks.push(json!({
            "type": "text",
            "text": "Transcribe the text from these document images exactly. Output ONLY the text. Do not add markdown formatting, commentary, or preambles."
        }));

        // Add images
        for img_bytes in images {
            let base64_image = BASE64_STANDARD.encode(img_bytes);
            content_blocks.push(json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/jpeg",
                    "data": base64_image
                }
            }));
        }

        let payload = json!({
            "model": config.model,
            "max_tokens": 4096,
            "messages": [
                {
                    "role": "user",
                    "content": content_blocks
                }
            ]
        });

        let response = client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic API error: {}", error_text));
        }

        let body: serde_json::Value = response.json().await?;
        
        let text = body["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }

    async fn transcribe_openai(
        client: &reqwest::Client,
        images: &[Vec<u8>],
        config: &ProviderAgentConfig,
    ) -> Result<String> {
        let mut content_blocks = Vec::new();

        content_blocks.push(json!({
            "type": "text",
            "text": "Transcribe the text from these document images exactly. Output ONLY the text. Do not add markdown formatting, commentary, or preambles."
        }));

        for img_bytes in images {
            let base64_image = BASE64_STANDARD.encode(img_bytes);
            content_blocks.push(json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:image/jpeg;base64,{}", base64_image)
                }
            }));
        }

        let payload = json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": content_blocks
                }
            ],
            "max_tokens": 4096
        });

        let response = client.post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let body: serde_json::Value = response.json().await?;
        
        let text = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}
