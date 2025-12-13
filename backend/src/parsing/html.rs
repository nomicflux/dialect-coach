use anyhow::{Result, anyhow};

pub fn parse_html(content: &str) -> Result<String> {
    if content.trim().is_empty() {
        return Err(anyhow!("Empty HTML content"));
    }

    // Simple heuristic: if it's too long, truncate clean logic handled in mod.rs
    // Here we just convert to text.
    // 12000 columns width to prevent hard wrapping (better for LLM)
    let text = html2text::from_read(content.as_bytes(), 12000);
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_basic() {
        let html = "<html><body><h1>Title</h1><p>Content</p></body></html>";
        let text = parse_html(html).unwrap();
        assert!(text.contains("Title"));
        assert!(text.contains("Content"));
    }
}
