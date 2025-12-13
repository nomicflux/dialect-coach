use anyhow::{Result, anyhow};

pub fn parse_text(content: &str) -> Result<String> {
    if content.trim().is_empty() {
        return Err(anyhow!("Empty text content"));
    }
    Ok(content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_empty() {
        assert!(parse_text("   ").is_err());
    }

    #[test]
    fn test_parse_text_valid() {
        assert_eq!(parse_text("hello").unwrap(), "hello");
    }
}
