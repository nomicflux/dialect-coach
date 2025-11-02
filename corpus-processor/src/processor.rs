use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use std::fs;
use std::path::Path;

use crate::chunking::{ChunkConfig, chunk_text};
use crate::embeddings::EmbeddingService;
use crate::loaders::load_corpus;

/// Process a corpus: load, chunk, embed, and save (with dependency injection)
pub fn process_corpus_with_embedder(
    input_path: &str,
    output_path: &str,
    dialect: Dialect,
    chunk_size: usize,
    overlap: usize,
    embedding_service: &EmbeddingService,
) -> Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_path).context(format!(
        "Failed to create output directory: {}",
        output_path
    ))?;

    // Check if already processed (look for any .jsonl file in output directory)
    let output_dir = std::path::Path::new(output_path);
    if output_dir.exists() {
        let existing_files: Vec<_> = std::fs::read_dir(output_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .collect();

        if !existing_files.is_empty() {
            let existing_file = &existing_files[0];
            let metadata = existing_file.metadata()?;
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

            println!(
                "  ⚠️  Already processed file found: {} ({:.1}MB)",
                existing_file.file_name().to_string_lossy(),
                size_mb
            );
            println!("  📝 Use 'rm -rf {}' to reprocess if needed", output_path);
            return Ok(());
        }
    }

    // Load corpus documents
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📂 STAGE 1/4: LOADING CORPUS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Input: {}", input_path);
    let raw_documents = load_corpus(input_path, dialect)?;
    println!("✅ Loaded {} documents", raw_documents.len());

    if raw_documents.is_empty() {
        println!("⚠️  Warning: No documents found in input path");
        return Ok(());
    }

    // Chunk documents with progress
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✂️  STAGE 2/4: CHUNKING TEXT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let chunk_config = ChunkConfig {
        max_chunk_size: chunk_size,
        overlap,
    };

    // Estimate chunks based on total content size
    let total_chars: usize = raw_documents.iter().map(|d| d.content.len()).sum();
    let estimated_chunks = total_chars / chunk_size + raw_documents.len();
    println!("Config: max_chunk_size={}, overlap={}", chunk_size, overlap);
    println!(
        "Total content: {:.1}MB ({} characters)",
        total_chars as f64 / (1024.0 * 1024.0),
        total_chars
    );
    println!("Estimated chunks: ~{}", estimated_chunks);
    println!("Processing {} documents...", raw_documents.len());

    let chunked_documents = chunk_documents(raw_documents, &chunk_config)?;
    println!("✅ Created {} chunks", chunked_documents.len());

    // Generate embeddings in batches with detailed progress
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧠 STAGE 3/4: GENERATING EMBEDDINGS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let total_chunks = chunked_documents.len();

    // Adaptive batch size based on dataset size - larger batches for big datasets
    let batch_size = if total_chunks > 10000 {
        64 // Larger batches for big datasets (2x faster)
    } else if total_chunks > 1000 {
        48 // Medium batch size
    } else {
        32 // Default batch size for small datasets
    };

    let total_batches = total_chunks.div_ceil(batch_size);

    println!("Total chunks: {}", total_chunks);
    println!("Batch size: {} chunks/batch", batch_size);
    println!("Total batches: {}", total_batches);

    let estimated_time_mins = (total_batches as f64 * 2.0) / 60.0;
    if estimated_time_mins > 1.0 {
        println!("⏱️  Estimated time: ~{:.1} minutes", estimated_time_mins);
    }
    println!();

    let mut processed_documents = Vec::new();
    let start_time = std::time::Instant::now();

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

        let elapsed_total = start_time.elapsed();
        let progress_percent = ((batch_idx + 1) as f64 / total_batches as f64) * 100.0;

        // Calculate ETA
        let avg_batch_time = elapsed_total.as_secs_f64() / (batch_idx + 1) as f64;
        let remaining_batches = total_batches - (batch_idx + 1);
        let eta_seconds = avg_batch_time * remaining_batches as f64;
        let eta_mins = eta_seconds / 60.0;

        // Show progress - every batch for small datasets, or strategically for large ones
        let should_print = total_batches <= 20 // Always print for small datasets
            || (batch_idx + 1) % 10 == 0 // Every 10 batches
            || batch_idx + 1 == total_batches // Last batch
            || (batch_idx + 1) % 100 == 0; // Every 100 batches

        if should_print {
            print!(
                "Progress: [{}/{}] {:.1}% | ",
                batch_idx + 1,
                total_batches,
                progress_percent
            );

            if remaining_batches > 0 {
                print!("ETA: {:.1}m | ", eta_mins);
            }

            print!(
                "Speed: {:.2}s/batch | Elapsed: {:.1}m | Total docs: {}",
                avg_batch_time,
                elapsed_total.as_secs_f64() / 60.0,
                processed_documents.len()
            );
            println!();
        }

        // Print milestone updates for large batches
        if total_batches > 50 && ((batch_idx + 1) % 100 == 0 || batch_idx + 1 == total_batches) {
            println!(
                "  🎯 Milestone: {}/{} batches completed ({:.1}%)",
                batch_idx + 1,
                total_batches,
                progress_percent
            );
        }
    }

    println!("✅ All embeddings generated");

    // Save processed documents
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💾 STAGE 4/4: SAVING DOCUMENTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Output directory: {}", output_path);
    println!("Documents to save: {}", processed_documents.len());

    let save_start = std::time::Instant::now();
    save_documents(&processed_documents, output_path)?;
    let save_duration = save_start.elapsed();

    let total_duration = start_time.elapsed();
    let file_size_mb = std::fs::metadata(
        std::path::Path::new(output_path).join(format!("{}.jsonl", dialect.id())),
    )?
    .len() as f64
        / (1024.0 * 1024.0);

    println!(
        "✅ Saved successfully ({:.2}s)",
        save_duration.as_secs_f64()
    );

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 PROCESSING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Dialect: {}", dialect.name());
    println!("Total documents: {}", processed_documents.len());
    println!("Output file size: {:.1}MB", file_size_mb);
    println!(
        "Total processing time: {:.1} minutes",
        total_duration.as_secs_f64() / 60.0
    );
    println!("Output location: {}", output_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

/// Process a corpus: load, chunk, embed, and save (convenience wrapper)
pub fn process_corpus(
    input_path: &str,
    output_path: &str,
    dialect: Dialect,
    chunk_size: usize,
    overlap: usize,
) -> Result<()> {
    let embedding_service = EmbeddingService::new()?;
    process_corpus_with_embedder(
        input_path,
        output_path,
        dialect,
        chunk_size,
        overlap,
        &embedding_service,
    )
}

/// Chunk documents into smaller pieces
pub fn chunk_documents(
    documents: Vec<DialectDocument>,
    config: &ChunkConfig,
) -> Result<Vec<DialectDocument>> {
    let mut chunked = Vec::new();

    for doc in documents {
        let chunks = chunk_text(&doc.content, config)?;

        for chunk_content in chunks {
            let chunk_doc = DialectDocument::new(chunk_content, doc.dialect, doc.formality);
            chunked.push(chunk_doc);
        }
    }

    Ok(chunked)
}

/// Save documents to output directory
pub fn save_documents(documents: &[DialectDocument], output_path: &str) -> Result<()> {
    let output_dir = Path::new(output_path);

    // Get dialect name from first document for filename
    let dialect_filename = if let Some(first_doc) = documents.first() {
        format!("{}.jsonl", first_doc.dialect.id())
    } else {
        "documents.jsonl".to_string()
    };

    // Save as a single JSONL file (JSON Lines format) with dialect-specific name
    let jsonl_path = output_dir.join(&dialect_filename);
    let mut lines = Vec::new();

    for doc in documents {
        let json = serde_json::to_string(doc).context("Failed to serialize document")?;
        lines.push(json);
    }

    fs::write(&jsonl_path, lines.join("\n")).context(format!(
        "Failed to write JSONL file: {}",
        jsonl_path.display()
    ))?;

    // Also save metadata summary with dialect-specific name
    let metadata_filename = if let Some(first_doc) = documents.first() {
        format!("{}_metadata.json", first_doc.dialect.id())
    } else {
        "metadata.json".to_string()
    };
    let metadata_path = output_dir.join(&metadata_filename);
    let metadata = serde_json::json!({
        "total_documents": documents.len(),
        "dialect": documents.first().map(|d| d.dialect.name()),
        "embedding_dimension": documents.first().map(|d| d.embedding.len()).unwrap_or(0),
        "formality_distribution": count_formality(documents),
    });

    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata).context("Failed to serialize metadata")?,
    )
    .context(format!(
        "Failed to write metadata file: {}",
        metadata_path.display()
    ))?;

    Ok(())
}

/// Count formality distribution
pub fn count_formality(documents: &[DialectDocument]) -> serde_json::Value {
    let mut formal = 0;
    let mut casual = 0;
    let mut dialect_rich = 0;
    let mut slang = 0;
    let mut unspecified = 0;

    for doc in documents {
        match doc.formality {
            Some(dialect_coach_shared::Formality::Formal) => formal += 1,
            Some(dialect_coach_shared::Formality::Casual) => casual += 1,
            Some(dialect_coach_shared::Formality::DialectRich) => dialect_rich += 1,
            Some(dialect_coach_shared::Formality::Slang) => slang += 1,
            None => unspecified += 1,
        }
    }

    serde_json::json!({
        "formal": formal,
        "casual": casual,
        "dialect_rich": dialect_rich,
        "slang": slang,
        "unspecified": unspecified,
    })
}
