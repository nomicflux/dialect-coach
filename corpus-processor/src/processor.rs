use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use futures::stream::{self, StreamExt};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

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

/// Concurrency limit for parallel LLM calls
const DEFAULT_CONCURRENCY: usize = 10;

/// Process a corpus: load, chunk, then for each chunk: enrich → embed → write (streaming)
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

    let total_count = documents_to_process.len();

    // Initialize LLM Client
    let llm_client = Arc::new(crate::llm::LlmClient::new()?);

    // Counters
    let written_count = Arc::new(AtomicUsize::new(0));
    let skipped_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    if dry_run {
        println!("enriching (dry run)... (concurrency: {})", concurrency);

        stream::iter(documents_to_process.iter().enumerate())
            .map(|(idx, doc)| {
                let llm = Arc::clone(&llm_client);
                let skipped = Arc::clone(&skipped_count);
                let errors = Arc::clone(&error_count);
                let written = Arc::clone(&written_count);
                let content = doc.content.clone();

                async move {
                    match llm.enrich_chunk(&content, dialect).await {
                        Ok(Some(data)) => {
                            let count = written.fetch_add(1, Ordering::Relaxed) + 1;
                            println!("[{}/{}] ✅ Chunk {}", count, total_count, idx + 1);
                            println!("  {:?}", data);
                        }
                        Ok(None) => {
                            skipped.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            errors.fetch_add(1, Ordering::Relaxed);
                            eprintln!("[Chunk {}] ❌ {}", idx + 1, e);
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        println!(
            "\n📊 Dry Run: {} enriched, {} skipped, {} errors",
            written_count.load(Ordering::Relaxed),
            skipped_count.load(Ordering::Relaxed),
            error_count.load(Ordering::Relaxed)
        );
        return Ok(());
    }

    // Real processing with channel-based streaming
    let file_name = format!("{}_enriched.jsonl", dialect.id());
    let path = Path::new(output_path).join(&file_name);

    println!(
        "processing (streaming to {})... (concurrency: {})",
        file_name, concurrency
    );

    // Channel for streaming: producer sends enriched chunks, consumer embeds and writes
    let (tx, mut rx) =
        mpsc::channel::<(usize, DialectDocument, crate::llm::EnrichedData)>(concurrency * 2);

    // Producer: parallel LLM calls, sends results to channel
    let producer = {
        let documents = documents_to_process.clone();
        let llm = Arc::clone(&llm_client);
        let skipped = Arc::clone(&skipped_count);
        let errors = Arc::clone(&error_count);

        tokio::spawn(async move {
            stream::iter(documents.into_iter().enumerate())
                .map(|(idx, doc)| {
                    let llm = Arc::clone(&llm);
                    let tx = tx.clone();
                    let skipped = Arc::clone(&skipped);
                    let errors = Arc::clone(&errors);

                    async move {
                        match llm.enrich_chunk(&doc.content, dialect).await {
                            Ok(Some(data)) => {
                                let _ = tx.send((idx, doc, data)).await;
                            }
                            Ok(None) => {
                                skipped.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(e) => {
                                errors.fetch_add(1, Ordering::Relaxed);
                                eprintln!("[Chunk {}] ❌ LLM: {}", idx + 1, e);
                            }
                        }
                    }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
        })
    };

    // Consumer: receives enriched chunks, embeds, writes to disk
    let file = fs::File::create(&path)?;
    let mut writer = BufWriter::new(file);

    while let Some((idx, doc, enriched)) = rx.recv().await {
        let context_text = enriched.context_triggers.join("\n");
        let keyword_text = enriched.keywords.join(" ");

        // Embed
        let content_emb = match embedding_service.embed_batch(vec![doc.content.clone()]) {
            Ok(mut v) => v.pop().unwrap(),
            Err(e) => {
                error_count.fetch_add(1, Ordering::Relaxed);
                eprintln!("[Chunk {}] ❌ Embed: {}", idx + 1, e);
                continue;
            }
        };

        let context_emb = if !context_text.is_empty() {
            embedding_service
                .embed_batch(vec![context_text])
                .ok()
                .and_then(|mut v| v.pop())
        } else {
            None
        };

        let keyword_emb = if !keyword_text.is_empty() {
            embedding_service
                .embed_batch(vec![keyword_text])
                .ok()
                .and_then(|mut v| v.pop())
        } else {
            None
        };

        let mut processed_doc = doc;
        processed_doc.embedding = content_emb;

        let record = EnrichedCorpusRecord {
            doc: processed_doc,
            context_emb,
            keyword_emb,
            enriched,
        };

        // Write immediately
        serde_json::to_writer(&mut writer, &record)?;
        writeln!(writer)?;

        let count = written_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(10) || count == 1 {
            writer.flush()?;
            println!("[{}/{}] ✅", count, total_count);
        }
    }

    writer.flush()?;
    producer.await?;

    println!(
        "\n✅ Complete: {} written, {} skipped, {} errors",
        written_count.load(Ordering::Relaxed),
        skipped_count.load(Ordering::Relaxed),
        error_count.load(Ordering::Relaxed)
    );
    println!("Saved to {}", path.display());

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
