pub mod html;
pub mod pdf;
pub mod text;

use anyhow::Result;

const MAX_CHARS: usize = 20_000;

pub enum DocumentSource {
    Text(String),
    Html(String),
    Pdf(Vec<u8>),
}

pub enum ParsedContent {
    Text(String),
    ScannedImages(Vec<Vec<u8>>),
}

pub fn extract_clean_text(source: DocumentSource) -> Result<ParsedContent> {
    match source {
        DocumentSource::Text(s) => Ok(ParsedContent::Text(text::parse_text(&s)?)),
        DocumentSource::Html(s) => Ok(ParsedContent::Text(truncate_text(&html::parse_html(&s)?))),
        DocumentSource::Pdf(b) => match pdf::parse_pdf(&b)? {
            ParsedContent::Text(t) => Ok(ParsedContent::Text(truncate_text(&t))),
            ParsedContent::ScannedImages(images) => Ok(ParsedContent::ScannedImages(images)),
        },
    }
}

fn truncate_text(text: &str) -> String {
    if text.len() <= MAX_CHARS {
        return text.to_string();
    }

    // Find the last paragraph break before MAX_CHARS
    let truncation_point = text
        .char_indices()
        .take(MAX_CHARS)
        .filter(|&(_, c)| c == '\n')
        .last()
        .map(|(i, _)| i)
        .unwrap_or(MAX_CHARS);

    let truncated = &text[..truncation_point];
    format!("{}\n\n[TRUNCATED: Document exceeded limits]", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_text() {
        let text = "Short text";
        assert_eq!(truncate_text(text), text);
    }

    #[test]
    fn test_truncate_long_text() {
        // Mock MAX_CHARS logic locally or relies on the const
        // We test the logic: it cuts.
        let long_text = "a".repeat(MAX_CHARS + 100);
        let truncated = truncate_text(&long_text);
        assert!(truncated.contains("[TRUNCATED"));
        assert!(truncated.len() < long_text.len());
    }
}
