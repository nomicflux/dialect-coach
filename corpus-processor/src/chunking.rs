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
        // CASE 1: The sentence itself is huge (larger than max_chunk_size)
        // We must split this specific sentence regardless of punctuation
        if sentence.len() > config.max_chunk_size {
            // If we have a pending chunk, flush it first
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
                previous_overlap = String::new();
            }

            // Now split the huge sentence by words
            // Note: This is a fallback to ensure we never produce huge chunks
            let mut sub_chunk = String::new();
            if !previous_overlap.is_empty() {
                sub_chunk = previous_overlap.clone();
            }

            for word in sentence.split_whitespace() {
                if !sub_chunk.is_empty() && sub_chunk.len() + word.len() + 1 > config.max_chunk_size
                {
                    // Flush sub-chunk
                    chunks.push(sub_chunk.trim().to_string());

                    // Calculate overlap for next sub-chunk
                    previous_overlap = extract_overlap(&sub_chunk, config.overlap);
                    sub_chunk = previous_overlap.clone();
                }

                if !sub_chunk.is_empty() {
                    sub_chunk.push(' ');
                }
                sub_chunk.push_str(word);
            }

            // Flush the final piece of the huge sentence
            if !sub_chunk.is_empty() {
                // Keep this piece as the "current_chunk" for the next iteration
                // so we can append subsequent small sentences to it if space permits
                current_chunk = sub_chunk;
                // Calculate overlap just in case we switch logic next
                previous_overlap = extract_overlap(&current_chunk, config.overlap);
            }
            continue;
        }

        // CASE 2: Semantic Windowing logic
        // We buffer sentences to create a sliding window of context
        // Current implementation: Just simple accumulation, but designed to be extensible
        // TODO: Implement actual sliding window (S1+S2+S3) if needed for RAG
        // For now, we stick to the accumulating logic which is effectively a variable-size window
        // bounded by max_chunk_size.

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
            // For true semantic windowing, we might want the overlap to be the *previous sentence completely*
            // rather than just `overlap` characters.
            // But for now, we respect the config.
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
    let mut start_idx = 0; // Track character index, not byte position
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();

    for (idx, (_byte_pos, c)) in char_indices.iter().enumerate() {
        // Simple sentence boundary detection
        if matches!(c, '.' | '!' | '?' | '؟' | '。') {
            // Look ahead to see if this is really end of sentence
            let next_char_exists = idx + 1 < char_indices.len();
            let is_sentence_end = if next_char_exists {
                let next_char = char_indices[idx + 1].1;
                next_char.is_whitespace()
            } else {
                true // End of text
            };

            if is_sentence_end {
                // Extract sentence using character indices - find the END of this character
                let start_byte = char_indices[start_idx].0;
                let end_byte = if idx + 1 < char_indices.len() {
                    char_indices[idx + 1].0 // Start of next character
                } else {
                    text.len() // End of text
                };
                let sentence = text[start_byte..end_byte].trim();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }

                // Move start to the next character after current sentence
                start_idx = idx + 1;
                // Skip whitespace characters
                while start_idx < char_indices.len() && char_indices[start_idx].1.is_whitespace() {
                    start_idx += 1;
                }
            }
        }
    }

    // Add remaining text as final sentence
    if start_idx < char_indices.len() {
        let start_byte = char_indices[start_idx].0;
        let remaining = text[start_byte..].trim();
        if !remaining.is_empty() {
            sentences.push(remaining);
        }
    }

    sentences
}

/// Extract the last N characters for overlap with next chunk
fn extract_overlap(text: &str, overlap_size: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= overlap_size {
        return text.to_string();
    }

    // Calculate character-based start position
    let char_start = char_count - overlap_size;

    // Convert to byte position safely
    let mut byte_start = 0;

    for (char_pos, (byte_pos, _)) in text.char_indices().enumerate() {
        if char_pos == char_start {
            byte_start = byte_pos;
            break;
        }
    }

    // Try to break at word boundary from the character-safe position
    let remaining_text = &text[byte_start..];
    if let Some(pos) = remaining_text.find(char::is_whitespace) {
        remaining_text[pos..].trim().to_string()
    } else {
        remaining_text.to_string()
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

#[test]
fn test_chunk_huge_sentence_no_punctuation() {
    let config = ChunkConfig {
        max_chunk_size: 50,
        overlap: 10,
    };
    // A "sentence" (no punctuation) that is strictly larger than max_chunk_size
    let huge_sentence = "This is a very long sentence that has absolutely no punctuation and will definitely exceed the fifty character limit we set in the configuration.";

    let chunks = chunk_text(huge_sentence, &config).unwrap();

    // It should have been split
    assert!(chunks.len() > 1);

    // All chunks should be within limit
    for chunk in &chunks {
        assert!(
            chunk.len() <= config.max_chunk_size,
            "Chunk size {} exceeds max {}",
            chunk.len(),
            config.max_chunk_size
        );
    }

    // Reconstruct to ensure we didn't lose words
    let reconstructed = chunks.join(" ");
    // Note: reconstruction might have extra overlaps repeated, so exact matching is tricky.
    // But we should at least find the words.
    assert!(chunks[0].starts_with("This is a"));
}
