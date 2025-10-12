use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use std::fs;
use std::path::Path;

use crate::chunking::{chunk_text, ChunkConfig};
use crate::embeddings::EmbeddingService;
use crate::loaders::load_corpus;

/// Process a corpus: load, chunk, embed, and save
pub fn process_corpus(
    input_path: &str,
    output_path: &str,
    dialect: Dialect,
    chunk_size: usize,
    overlap: usize,
) -> Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_path)
        .context(format!("Failed to create output directory: {}", output_path))?;

    // Load corpus documents
    println!("Loading corpus from {}...", input_path);
    let raw_documents = load_corpus(input_path, dialect.clone())?;
    println!("Loaded {} documents", raw_documents.len());

    if raw_documents.is_empty() {
        println!("Warning: No documents found in input path");
        return Ok(());
    }

    // Chunk documents
    println!("Chunking documents...");
    let chunk_config = ChunkConfig {
        max_chunk_size: chunk_size,
        overlap,
    };
    let chunked_documents = chunk_documents(raw_documents, &chunk_config)?;
    println!("Created {} chunks", chunked_documents.len());

    // Initialize embedding service
    let embedding_service = EmbeddingService::new()?;

    // Generate embeddings in batches
    println!("Generating embeddings...");
    let batch_size = 32;
    let mut processed_documents = Vec::new();

    for (batch_idx, chunk) in chunked_documents.chunks(batch_size).enumerate() {
        let texts: Vec<String> = chunk.iter().map(|doc| doc.content.clone()).collect();

        let embeddings = embedding_service
            .embed_batch(texts)
            .context("Failed to generate embeddings for batch")?;

        // Update documents with embeddings
        for (doc, embedding) in chunk.iter().zip(embeddings.into_iter()) {
            let mut processed_doc = doc.clone();
            processed_doc.embedding = embedding;
            processed_documents.push(processed_doc);
        }

        println!(
            "  Processed batch {}/{} ({} documents)",
            batch_idx + 1,
            (chunked_documents.len() + batch_size - 1) / batch_size,
            processed_documents.len()
        );
    }

    // Save processed documents
    println!("Saving processed documents...");
    save_documents(&processed_documents, output_path)?;

    println!("Saved {} documents to {}", processed_documents.len(), output_path);

    Ok(())
}

/// Chunk documents into smaller pieces
fn chunk_documents(
    documents: Vec<DialectDocument>,
    config: &ChunkConfig,
) -> Result<Vec<DialectDocument>> {
    let mut chunked = Vec::new();

    for doc in documents {
        let chunks = chunk_text(&doc.content, config)?;

        for chunk_content in chunks {
            let chunk_doc = DialectDocument::new(
                chunk_content,
                doc.dialect.clone(),
                doc.formality.clone(),
            );
            chunked.push(chunk_doc);
        }
    }

    Ok(chunked)
}

/// Save documents to output directory
fn save_documents(documents: &[DialectDocument], output_path: &str) -> Result<()> {
    let output_dir = Path::new(output_path);

    // Save as a single JSONL file (JSON Lines format)
    let jsonl_path = output_dir.join("documents.jsonl");
    let mut lines = Vec::new();

    for doc in documents {
        let json = serde_json::to_string(doc)
            .context("Failed to serialize document")?;
        lines.push(json);
    }

    fs::write(&jsonl_path, lines.join("\n"))
        .context(format!("Failed to write JSONL file: {}", jsonl_path.display()))?;

    // Also save metadata summary
    let metadata_path = output_dir.join("metadata.json");
    let metadata = serde_json::json!({
        "total_documents": documents.len(),
        "dialect": documents.first().map(|d| d.dialect.name()),
        "embedding_dimension": documents.first().map(|d| d.embedding.len()).unwrap_or(0),
        "formality_distribution": count_formality(documents),
    });

    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata)
            .context("Failed to serialize metadata")?,
    )
    .context(format!("Failed to write metadata file: {}", metadata_path.display()))?;

    Ok(())
}

/// Count formality distribution
fn count_formality(documents: &[DialectDocument]) -> serde_json::Value {
    let mut formal = 0;
    let mut casual = 0;
    let mut slang = 0;
    let mut unspecified = 0;

    for doc in documents {
        match doc.formality {
            Some(dialect_coach_shared::Formality::Formal) => formal += 1,
            Some(dialect_coach_shared::Formality::Casual) => casual += 1,
            Some(dialect_coach_shared::Formality::Slang) => slang += 1,
            None => unspecified += 1,
        }
    }

    serde_json::json!({
        "formal": formal,
        "casual": casual,
        "slang": slang,
        "unspecified": unspecified,
    })
}
