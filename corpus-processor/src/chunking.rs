use anyhow::Result;

/// Configuration for text chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum chunk size in characters
    pub max_chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 512,
            overlap: 50,
        }
    }
}

/// Chunk text into overlapping segments
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Result<Vec<String>> {
    let mut chunks = Vec::new();

    if text.is_empty() {
        return Ok(chunks);
    }

    // Split into sentences first (basic approach)
    let sentences = split_into_sentences(text);

    let mut current_chunk = String::new();
    let mut previous_overlap = String::new();

    for sentence in sentences {
        // Start new chunk with overlap from previous
        if current_chunk.is_empty() && !previous_overlap.is_empty() {
            current_chunk = previous_overlap.clone();
        }

        // Check if adding this sentence would exceed max size
        if !current_chunk.is_empty() && current_chunk.len() + sentence.len() > config.max_chunk_size
        {
            // Save current chunk
            chunks.push(current_chunk.trim().to_string());

            // Prepare overlap for next chunk
            previous_overlap = extract_overlap(&current_chunk, config.overlap);

            // Start new chunk
            current_chunk = String::new();
        }

        // Add sentence to current chunk
        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(sentence);
    }

    // Add final chunk if not empty
    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    Ok(chunks)
}

/// Split text into sentences (basic implementation)
fn split_into_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;

    for (i, c) in text.char_indices() {
        // Simple sentence boundary detection
        if matches!(c, '.' | '!' | '?' | '؟' | '。') {
            // Look ahead to see if this is really end of sentence
            let next_chars: String = text[i + 1..].chars().take(2).collect();
            if next_chars.starts_with(char::is_whitespace) || next_chars.is_empty() {
                sentences.push(text[start..=i].trim());
                start = i + 1;
            }
        }
    }

    // Add remaining text as final sentence
    if start < text.len() {
        let remaining = text[start..].trim();
        if !remaining.is_empty() {
            sentences.push(remaining);
        }
    }

    sentences
}

/// Extract the last N characters for overlap with next chunk
fn extract_overlap(text: &str, overlap_size: usize) -> String {
    if text.len() <= overlap_size {
        return text.to_string();
    }

    // Try to break at word boundary
    let start = text.len() - overlap_size;
    if let Some(pos) = text[start..].find(char::is_whitespace) {
        text[start + pos..].trim().to_string()
    } else {
        text[start..].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_empty_text() {
        let config = ChunkConfig::default();
        let chunks = chunk_text("", &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_short_text() {
        let config = ChunkConfig::default();
        let text = "This is a short text.";
        let chunks = chunk_text(text, &config).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_chunk_with_overlap() {
        let config = ChunkConfig {
            max_chunk_size: 50,
            overlap: 10,
        };
        let text =
            "This is the first sentence. This is the second sentence. This is the third sentence.";
        let chunks = chunk_text(text, &config).unwrap();
        assert!(chunks.len() > 1);

        // Verify chunks have content
        for chunk in &chunks {
            assert!(!chunk.is_empty());
        }
    }

    #[test]
    fn test_sentence_splitting() {
        let sentences = split_into_sentences("First. Second! Third?");
        assert_eq!(sentences.len(), 3);
    }
}
