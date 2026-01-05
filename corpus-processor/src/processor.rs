use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use futures::stream::{self, StreamExt};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub type EnrichedCorpusTuple = (
    DialectDocument,
    Option<Vec<f32>>,
    Option<Vec<f32>>,
    crate::llm::EnrichedData,
);

pub struct ProcessorConfig<'a> {
    pub input_path: &'a str,
    pub output_path: &'a str,
    pub dialect: Dialect,
    pub chunk_size: usize,
    pub overlap: usize,
    pub max_chunks: Option<usize>,
    pub dry_run: bool,
    pub concurrency: usize,
}

use crate::chunking::{chunk_text, ChunkConfig};
use crate::embeddings::EmbeddingService;
use crate::loaders::load_corpus;
use std::io::Write;

/// Concurrency limit for parallel LLM calls
const DEFAULT_CONCURRENCY: usize = 10;

/// Process a corpus: load, chunk, enrich (LLM), embed, and save
pub async fn process_corpus_with_embedder(
    config: ProcessorConfig<'_>,
    embedding_service: &EmbeddingService,
) -> Result<()> {
    let input_path = config.input_path;
    let output_path = config.output_path;
    let dialect = config.dialect;
    let chunk_size = config.chunk_size;
    let overlap = config.overlap;
    let max_chunks = config.max_chunks;
    let dry_run = config.dry_run;
    let concurrency = config.concurrency;

    fs::create_dir_all(output_path).context(format!(
        "Failed to create output directory: {}",
        output_path
    ))?;

    let output_dir = Path::new(output_path);
    if output_dir.exists() {
        let existing_files: Vec<_> = std::fs::read_dir(output_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        if !existing_files.is_empty() && !dry_run {
            println!("⚠️  Output files exist. Please clear output directory to reprocess.");
        }
    }

    println!("loading corpus...");
    let raw_documents = load_corpus(input_path, dialect)?;
    if raw_documents.is_empty() {
        return Ok(());
    }

    println!("chunking...");
    let chunk_config = ChunkConfig {
        max_chunk_size: chunk_size,
        overlap,
    };
    let chunked_documents = crate::processor::chunk_documents(raw_documents, &chunk_config)?;
    println!("created {} chunks", chunked_documents.len());

    let documents_to_process: Vec<_> = if let Some(max) = max_chunks {
        if max < chunked_documents.len() {
            println!(
                "⚠️  Limiting processing to first {} chunks (via --max-chunks)",
                max
            );
            chunked_documents.into_iter().take(max).collect()
        } else {
            chunked_documents
        }
    } else {
        chunked_documents
    };

    // Initialize LLM Client
    let llm_client = Arc::new(crate::llm::LlmClient::new()?);

    println!(
        "enriching and embedding... (concurrency: {})",
        concurrency
    );

    // Counters for progress tracking
    let processed_count = Arc::new(AtomicUsize::new(0));
    let skipped_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let total_count = documents_to_process.len();

    // Phase 1: Parallel LLM enrichment
    let enrichment_results: Vec<_> = stream::iter(documents_to_process.iter().enumerate())
        .map(|(idx, doc)| {
            let llm = Arc::clone(&llm_client);
            let processed = Arc::clone(&processed_count);
            let skipped = Arc::clone(&skipped_count);
            let errors = Arc::clone(&error_count);
            let content = doc.content.clone();

            async move {
                let result = llm.enrich_chunk(&content, dialect).await;

                match result {
                    Ok(Some(enriched)) => {
                        let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                        println!(
                            "[{}/{}] ✅ Enriched chunk {}",
                            count,
                            total_count,
                            idx + 1
                        );
                        Some((idx, enriched))
                    }
                    Ok(None) => {
                        skipped.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    Err(e) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[Chunk {}] ❌ LLM Error: {}", idx + 1, e);
                        None
                    }
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let enriched_chunks: Vec<_> = enrichment_results.into_iter().flatten().collect();

    println!(
        "\n📊 LLM Phase Complete: {} enriched, {} skipped, {} errors",
        processed_count.load(Ordering::Relaxed),
        skipped_count.load(Ordering::Relaxed),
        error_count.load(Ordering::Relaxed)
    );

    if dry_run {
        for (idx, enriched) in &enriched_chunks {
            let doc = &documents_to_process[*idx];
            let context_text = enriched.context_triggers.join("\n");
            let keyword_text = enriched.keywords.join(" ");
            println!("\n[DRY RUN] Chunk {}", idx);
            println!("  METADATA: {:?}", enriched);
            println!("  VECTOR SOURCE DATA:");
            println!("    ► Content (to embed): {:?}", doc.content);
            println!("    ► Context (to embed): {:?}", context_text);
            println!("    ► Keywords (to embed): {:?}", keyword_text);
        }
        println!("\nDry run complete. No data saved/uploaded.");
        return Ok(());
    }

    // Phase 2: Sequential embeddings (fastembed is CPU-bound, not async-friendly)
    println!("\n🔄 Generating embeddings...");
    let mut results = Vec::new();

    for (idx, enriched) in enriched_chunks {
        let doc = &documents_to_process[idx];
        let context_text = enriched.context_triggers.join("\n");
        let keyword_text = enriched.keywords.join(" ");

        // Embed Content
        let content_emb = embedding_service
            .embed_batch(vec![doc.content.clone()])?
            .pop()
            .unwrap();

        // Embed Context (Triggers)
        let context_emb = if !context_text.is_empty() {
            Some(
                embedding_service
                    .embed_batch(vec![context_text])?
                    .pop()
                    .unwrap(),
            )
        } else {
            None
        };

        // Embed Keywords
        let keyword_emb = if !keyword_text.is_empty() {
            Some(
                embedding_service
                    .embed_batch(vec![keyword_text])?
                    .pop()
                    .unwrap(),
            )
        } else {
            None
        };

        let mut processed_doc = doc.clone();
        processed_doc.embedding = content_emb;

        results.push((processed_doc, context_emb, keyword_emb, enriched));
    }

    println!("✅ Generated {} total embeddings", results.len());

    // Save to disk
    let records: Vec<EnrichedCorpusRecord> = results
        .into_iter()
        .map(|(doc, ce, ke, en)| EnrichedCorpusRecord {
            doc,
            context_emb: ce,
            keyword_emb: ke,
            enriched: en,
        })
        .collect();

    let file_name = format!("{}_enriched.jsonl", dialect.id());
    let path = Path::new(output_path).join(file_name);
    let mut file = fs::File::create(&path)?;
    for record in records {
        serde_json::to_writer(&file, &record)?;
        writeln!(file)?;
    }
    println!("Saved enriched data to {}", path.display());

    Ok(())
}

/// Record format for enriched corpus data
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EnrichedCorpusRecord {
    pub doc: DialectDocument,
    pub context_emb: Option<Vec<f32>>,
    pub keyword_emb: Option<Vec<f32>>,
    pub enriched: crate::llm::EnrichedData,
}

/// Process corpus wrapper
pub async fn process_corpus(
    input_path: &str,
    output_path: &str,
    dialect: Dialect,
    chunk_size: usize,
    overlap: usize,
    max_chunks: Option<usize>,
    dry_run: bool,
) -> Result<()> {
    let embedding_service = EmbeddingService::new()?;
    let config = ProcessorConfig {
        input_path,
        output_path,
        dialect,
        chunk_size,
        overlap,
        max_chunks,
        dry_run,
        concurrency: DEFAULT_CONCURRENCY,
    };

    process_corpus_with_embedder(config, &embedding_service).await
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
