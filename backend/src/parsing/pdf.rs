use anyhow::{Result, anyhow};


pub fn parse_pdf(bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() {
        return Err(anyhow!("Empty PDF content"));
    }

    // pdf-extractOutput is Result<String>
    // It extracts text from all pages.
    // We rely on the library to handle basic extraction.
    // Length limits are applied in the coordinator.
    pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| anyhow!("Failed to extract PDF text: {}", e))
}

#[cfg(test)]
mod tests {
    // PDF tests are hard without a real file.
    // We trust the library, integration tests will cover this with a mock/sample if needed.
    // Or we skip unit testing this wrapper for now as it just calls the lib.
}
