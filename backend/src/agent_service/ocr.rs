use super::provider::CompletionAgent;
use anyhow::{Context, Result};

pub struct OcrService;

impl OcrService {
    pub async fn transcribe_images(
        images: &[Vec<u8>],
        agent: &dyn CompletionAgent,
    ) -> Result<String> {
        let prompt = "Transcribe the text from these document images exactly. Output ONLY the text. Do not add markdown formatting, commentary, or preambles.";
        Ok(agent
            .complete_images(prompt, images, 4096)
            .await
            .with_context(|| format!("OCR failed for {}", agent.model()))?
            .text)
    }
}
