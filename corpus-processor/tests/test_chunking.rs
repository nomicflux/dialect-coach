use corpus_processor::chunking::{chunk_text, ChunkConfig};

#[test]
fn test_empty_text() {
    let config = ChunkConfig::default();
    let chunks = chunk_text("", &config).unwrap();
    assert!(chunks.is_empty());
}

#[test]
fn test_short_text() {
    let config = ChunkConfig::default();
    let text = "This is a short text.";
    let chunks = chunk_text(text, &config).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
}

#[test]
fn test_chunk_boundaries() {
    let config = ChunkConfig {
        max_chunk_size: 50,
        overlap: 10,
    };

    // Text exactly at boundary
    let text = "A".repeat(50);
    let chunks = chunk_text(&text, &config).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].len(), 50);

    // Text just over boundary
    let text = "A".repeat(51);
    let chunks = chunk_text(&text, &config).unwrap();
    assert!(!chunks.is_empty());
}

#[test]
fn test_unicode_safety() {
    let config = ChunkConfig {
        max_chunk_size: 100,
        overlap: 20,
    };

    let arabic_text = "مرحبا بك في المغرب. هذا نص باللغة العربية. نحن نختبر التقسيم النصي.";
    let chunks = chunk_text(arabic_text, &config).unwrap();

    assert!(!chunks.is_empty());

    // Verify each chunk is valid UTF-8
    for chunk in &chunks {
        assert!(chunk.is_char_boundary(chunk.len()));
        assert!(!chunk.is_empty());
    }
}

#[test]
fn test_sentence_boundaries() {
    let config = ChunkConfig {
        max_chunk_size: 30,
        overlap: 5,
    };

    let text = "First sentence. Second sentence! Third question?";
    let chunks = chunk_text(text, &config).unwrap();

    // Should have multiple chunks due to size limit
    assert!(chunks.len() > 1);

    for chunk in &chunks {
        assert!(!chunk.trim().is_empty());
    }
}

#[test]
fn test_arabic_punctuation() {
    let config = ChunkConfig {
        max_chunk_size: 50,
        overlap: 10,
    };

    let text = "هذا سؤال؟ هذا جملة أخرى. نهاية النص.";
    let chunks = chunk_text(text, &config).unwrap();

    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.is_empty());
    }
}

#[test]
fn test_overlap_functionality() {
    let config = ChunkConfig {
        max_chunk_size: 40,
        overlap: 10,
    };

    let text = "This is a long text that will definitely be split into multiple chunks by the chunking algorithm.";
    let chunks = chunk_text(text, &config).unwrap();

    if chunks.len() > 1 {
        // There should be some overlap between consecutive chunks
        // This is hard to test precisely without knowing internal algorithm,
        // but we can verify basic properties
        for chunk in &chunks {
            assert!(chunk.len() <= config.max_chunk_size);
        }
    }
}

#[test]
fn test_mixed_scripts() {
    let config = ChunkConfig::default();

    let mixed_text = "Hello مرحبا 你好 emoji: 😀 more text here.";
    let chunks = chunk_text(mixed_text, &config).unwrap();

    assert!(!chunks.is_empty());
    assert_eq!(chunks[0], mixed_text); // Should fit in one chunk
}

#[test]
fn test_chunk_config_default() {
    let config = ChunkConfig::default();
    assert_eq!(config.max_chunk_size, 512);
    assert_eq!(config.overlap, 50);
}

#[test]
fn test_extract_overlap_edge_cases() {
    let config = ChunkConfig {
        max_chunk_size: 20,
        overlap: 15,
    };

    // Test overlap larger than text - should return whole text
    let short_text = "Short";
    let chunks = chunk_text(short_text, &config).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], short_text);
}

#[test]
fn test_sentence_boundary_edge_cases() {
    let config = ChunkConfig {
        max_chunk_size: 30,
        overlap: 5,
    };

    // Test punctuation without following whitespace
    let text = "First.Second!Third";
    let chunks = chunk_text(text, &config).unwrap();
    assert!(!chunks.is_empty());

    // Test punctuation at end of text
    let text = "This is a sentence.";
    let chunks = chunk_text(text, &config).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "This is a sentence.");
}

#[test]
fn test_zero_overlap() {
    let config = ChunkConfig {
        max_chunk_size: 20,
        overlap: 0,
    };

    let text = "First sentence. Second sentence. Third sentence.";
    let chunks = chunk_text(text, &config).unwrap();
    assert!(!chunks.is_empty());

    // With zero overlap, no text should be duplicated between chunks
    for chunk in &chunks {
        assert!(!chunk.is_empty());
        assert!(chunk.len() <= config.max_chunk_size);
    }
}

#[test]
fn test_chinese_punctuation() {
    let config = ChunkConfig {
        max_chunk_size: 50,
        overlap: 10,
    };

    // Test Chinese sentence ending punctuation
    let text = "这是第一句。这是第二句。这是第三句。";
    let chunks = chunk_text(text, &config).unwrap();
    assert!(!chunks.is_empty());

    for chunk in &chunks {
        assert!(!chunk.is_empty());
    }
}

#[test]
fn test_whitespace_only_sentences() {
    let config = ChunkConfig::default();

    // Test text that produces empty sentences after trimming
    let text = "First.   \n\t  Second.";
    let chunks = chunk_text(text, &config).unwrap();
    assert!(!chunks.is_empty());

    for chunk in &chunks {
        assert!(!chunk.trim().is_empty());
    }
}
