use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use std::fs;
use std::path::Path;

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
}

use crate::chunking::{chunk_text, ChunkConfig};
use crate::embeddings::EmbeddingService;
use crate::loaders::load_corpus;
use std::io::Write;

/// Process a corpus: load, chunk, embed, and save (with dependency injection)
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
    // [Setup output dir omitted, assuming managed by caller or we add it back if I deleted it?
    // Wait, I am replacing the whole function. I must keep the directory creation]
    fs::create_dir_all(output_path).context(format!(
        "Failed to create output directory: {}",
        output_path
    ))?;

    // ... [existing check for processed file code could be here, but for brevity/cleanliness and strictly following the "rebuild" plan I will skip complex "resume" logic for now and focus on the new flow]
    // Actually, good to keep it.
    let output_dir = Path::new(output_path);
    if output_dir.exists() {
        let existing_files: Vec<_> = std::fs::read_dir(output_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        if !existing_files.is_empty() && !dry_run {
            println!("⚠️  Output files exist. Please clear output directory to reprocess.");
            // return Ok(()); // Force user to clean up? Or just proceed?
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

    let documents_to_process = if let Some(max) = max_chunks {
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
    let llm_client = crate::llm::LlmClient::new()?;

    // Results container
    // We store: (DialectDocument (with content embedding), ContextEmb, KeywordEmb, EnrichedData)
    let mut results = Vec::new();

    println!("enriching and embedding...");
    // We process sequentially or semi-parallel? Llm is slow.
    // Sequentially for now to be safe and simple
    let mut skipped_count = 0;

    for (idx, doc) in documents_to_process.iter().enumerate() {
        println!("[Chunk {}] ⏳ 1. Requesting LLM Enrichment...", idx + 1);
        let start_time = std::time::Instant::now();

        // 1. Enrich
        let enriched_opt = match llm_client.enrich_chunk(&doc.content, dialect).await {
            Ok(res) => {
                println!(
                    "[Chunk {}] ✅ 2. LLM Responded ({:.2?})",
                    idx + 1,
                    start_time.elapsed()
                );
                res
            }
            Err(e) => {
                println!("\n❌ LLM Error on chunk {}: {}", idx + 1, e);
                continue; // Skip failed LLM calls?
            }
        };

        let enriched = match enriched_opt {
            Some(data) => data,
            None => {
                skipped_count += 1;
                continue; // Filtered out
            }
        };

        // Prepare text for embedding
        let context_text = enriched.context_triggers.join("\n");
        let keyword_text = enriched.keywords.join(" ");

        if dry_run {
            println!("\n[DRY RUN] Chunk {}", idx);
            println!("  METADATA: {:?}", enriched);
            println!("  VECTOR SOURCE DATA:");
            println!("    ► Content (to embed): {:?}", doc.content);
            println!("    ► Context (to embed): {:?}", context_text);
            println!("    ► Keywords (to embed): {:?}", keyword_text);
            continue;
        }

        // 2. Embed Content
        println!("[Chunk {}] ⏳ 3. Generating Embeddings...", idx + 1);
        let content_emb = embedding_service
            .embed_batch(vec![doc.content.clone()])?
            .pop()
            .unwrap();

        // 3. Embed Context (Triggers)
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

        // 4. Embed Keywords
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
        processed_doc.embedding = content_emb; // Store content embedding in doc as usual

        results.push((processed_doc, context_emb, keyword_emb, enriched));
        println!("[Chunk {}] ✅ 4. Done.", idx + 1);
    }

    println!("\nSkipped {} chunks due to filtration.", skipped_count);

    if dry_run {
        println!("Dry run complete. No data saved/uploaded.");
        return Ok(());
    }

    // Connect Qdrant and Upload
    // We need Qdrant URL/Key here.
    // Wait, process_corpus signature didn't have URL/Key.
    // The previous implementation SAVED to disk, then `Upload` command uploaded.
    // The NEW plan implies doing it all?
    // "Modify qdrant.rs to update init_collection ... and upload_documents".
    // "Rebuild the corpus: Delete all ... and re-process/upload".

    // The `Process` command in CLI saves to JSONL.
    // The `Upload` command uploads.
    // I should persist the `EnrichedData` and `ExtraEmbeddings` to disk so `Upload` can pick them up?
    // OR change `Process` to do everything?
    // The previous code separated them.
    // If I want to support `Upload` command using `EnrichedData`, I need to serialize `EnrichedData` to the JSONL.

    // Let's UPDATE `DialectDocument` serialization strategy?
    // Or save a new file format.
    // The CLI `Upload` command reads `load_documents_from_jsonl`.

    // I will Save `results` to a new JSONL format that includes everything.
    // Then I need to update `Upload` command to read it and call `upload_enriched_documents`.

    // Let's define a wrapper struct for serialization
    // MOVED to module level to be public
    let records: Vec<EnrichedCorpusRecord> = results
        .into_iter()
        .map(|(doc, ce, ke, en)| EnrichedCorpusRecord {
            doc,
            context_emb: ce,
            keyword_emb: ke,
            enriched: en,
        })
        .collect();

    // Save to disk
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
