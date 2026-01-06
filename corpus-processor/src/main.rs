mod chunking;
mod embeddings;
mod llm;
mod loaders;
mod processor;
mod qdrant;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialect_coach_shared::{Dialect, Language};
use std::fs;
use std::io::{BufRead, BufReader};

#[derive(Parser)]
#[command(name = "corpus-processor")]
#[command(about = "Process dialect corpus data for RAG", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Load corpus files and generate embeddings
    Process {
        /// Language to process
        #[arg(short, long)]
        language: String,

        /// Dialect to process
        #[arg(short, long)]
        dialect: String,

        /// Input corpus file or directory
        #[arg(short, long)]
        input: String,

        /// Output directory for processed documents
        #[arg(short, long, default_value = "output")]
        output: String,

        /// Maximum chunk size in characters
        #[arg(long, default_value = "512")]
        chunk_size: usize,

        #[arg(long, default_value = "50")]
        overlap: usize,

        /// Stop after processing this many chunks (useful for testing)
        #[arg(long)]
        max_chunks: Option<usize>,

        /// Dry run: enrich/filter but do NOT embed/save
        #[arg(long)]
        dry_run: bool,
    },

    Delete {
        #[arg(short, long)]
        dialect: String,
        #[arg(short, long)]
        url: Option<String>,
        #[arg(short = 'k', long)]
        api_key: Option<String>,
    },

    /// Upload processed documents to Qdrant
    Upload {
        /// Path to processed documents (JSONL file)
        #[arg(short, long)]
        input: String,

        /// Qdrant server URL (can also use QDRANT_URL env var)
        #[arg(short, long)]
        url: Option<String>,

        /// Qdrant API key (optional, can also use QDRANT_API_KEY env var)
        #[arg(short = 'k', long)]
        api_key: Option<String>,
    },

    /// List available dialects
    List,

    Update {
        #[arg(short = 'f', long = "dialect-from")]
        dialect_from: String,
        #[arg(short = 't', long = "dialect-to")]
        dialect_to: String,
        #[arg(short = 'u', long = "url")]
        url: Option<String>,
        #[arg(short = 'k', long = "key")]
        api_key: Option<String>,
    },

    /// Check status of Qdrant collection and loaded dialects
    Status {
        /// Qdrant server URL (can also use QDRANT_URL env var)
        #[arg(short, long)]
        url: Option<String>,

        /// Qdrant API key (optional, can also use QDRANT_API_KEY env var)
        #[arg(short = 'k', long)]
        api_key: Option<String>,
    },

    /// Reset (Delete & Recreate) the Qdrant collection. WARNING: Destructive!
    ResetCollection {
        /// Qdrant server URL (can also use QDRANT_URL env var)
        #[arg(short, long)]
        url: Option<String>,

        /// Qdrant API key (optional, can also use QDRANT_API_KEY env var)
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

fn get_qdrant_url(url: Option<String>) -> Result<String> {
    url.or_else(|| std::env::var("QDRANT_URL").ok().filter(|s| !s.is_empty()))
        .context("QDRANT_URL must be provided via --url flag or QDRANT_URL environment variable")
}

fn get_qdrant_key(api_key: Option<String>) -> Option<String> {
    api_key.or_else(|| std::env::var("QDRANT_API_KEY").ok())
}

async fn get_qdrant_service(
    url: Option<String>,
    api_key: Option<String>,
) -> Result<qdrant::QdrantService> {
    let qdrant_url = get_qdrant_url(url)?;
    println!("Connecting to: {}", qdrant_url);
    let qdrant_api_key = get_qdrant_key(api_key);
    if let Some(key) = qdrant_api_key {
        qdrant::QdrantService::new_with_api_key(&qdrant_url, &key).await
    } else {
        qdrant::QdrantService::new(&qdrant_url).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Process {
            language,
            dialect,
            input,
            output,
            chunk_size,
            overlap,
            max_chunks,
            dry_run,
        } => {
            println!("Processing corpus:");
            println!("  Language: {}", language);
            println!("  Dialect: {}", dialect);
            println!("  Input: {}", input);
            println!("  Output: {}", output);
            println!("  Chunk size: {}", chunk_size);
            println!("  Overlap: {}", overlap);
            if let Some(max) = max_chunks {
                println!("  Max chunks: {}", max);
            }
            println!();

            // Parse dialect
            let dialect_enum = parse_dialect(&language, &dialect)?;

            // Process the corpus
            processor::process_corpus(
                &input,
                &output,
                dialect_enum,
                chunk_size,
                overlap,
                max_chunks,
                dry_run,
            )
            .await?;

            println!("\n✓ Processing completed successfully");
        }
        Commands::Delete {
            dialect,
            url,
            api_key,
        } => {
            println!("Deleting points from Qdrant:");
            println!("  Dialect: {}", dialect);
            let qdrant = get_qdrant_service(url, api_key).await?;
            qdrant.delete_points(&dialect).await?;
        }
        Commands::Upload {
            input,
            url,
            api_key,
        } => {
            println!("Uploading documents to Qdrant (streaming):");
            println!("  Input: {}\n", input);

            // Connect to Qdrant first
            let qdrant = get_qdrant_service(url, api_key).await?;

            // Count total lines for progress
            let line_count = {
                let file = fs::File::open(&input)
                    .context(format!("Failed to open file: {}", input))?;
                BufReader::new(file).lines().count()
            };
            println!("📊 Total records in file: {}", line_count);

            // Initialize collection (get vector size from first record)
            let file = fs::File::open(&input)?;
            let mut reader = BufReader::new(file);
            let mut first_line = String::new();
            reader.read_line(&mut first_line)?;
            if !first_line.trim().is_empty() {
                let first_rec: crate::processor::EnrichedCorpusRecord =
                    serde_json::from_str(&first_line).context("Failed to parse first record")?;
                let vector_size = first_rec.doc.embedding.len() as u64;
                qdrant.init_collection(vector_size).await?;
            }

            // Stream and upload in batches
            let file = fs::File::open(&input)?;
            let reader = BufReader::new(file);
            
            const BATCH_SIZE: usize = 100;
            let mut batch: Vec<crate::processor::EnrichedCorpusTuple> = Vec::with_capacity(BATCH_SIZE);
            let mut uploaded_count = 0usize;
            let mut error_count = 0usize;

            for (line_num, line_result) in reader.lines().enumerate() {
                let line = match line_result {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("[Line {}] ❌ Read error: {}", line_num + 1, e);
                        error_count += 1;
                        continue;
                    }
                };

                if line.trim().is_empty() {
                    continue;
                }

                let rec: crate::processor::EnrichedCorpusRecord = match serde_json::from_str(&line) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[Line {}] ❌ Parse error: {}", line_num + 1, e);
                        error_count += 1;
                        continue;
                    }
                };

                batch.push((rec.doc, rec.context_emb, rec.keyword_emb, rec.enriched));

                // Upload when batch is full
                if batch.len() >= BATCH_SIZE {
                    if let Err(e) = qdrant.upload_batch(&batch).await {
                        eprintln!("[Batch] ❌ Upload error: {:?}", e);
                        error_count += batch.len();
                    } else {
                        uploaded_count += batch.len();
                    }
                    
                    println!("[{}/{}] ✅ Uploaded {} records", uploaded_count, line_count, uploaded_count);
                    batch.clear();
                }
            }

            // Upload remaining records
            if !batch.is_empty() {
                if let Err(e) = qdrant.upload_batch(&batch).await {
                    eprintln!("[Batch] ❌ Upload error: {:?}", e);
                    error_count += batch.len();
                } else {
                    uploaded_count += batch.len();
                }
            }

            println!("\n✅ Upload complete: {} uploaded, {} errors", uploaded_count, error_count);
        }
        Commands::List => {
            println!("Available languages and dialects:\n");

            for lang in Language::all() {
                println!("{}:", lang.name());
                let dialects = Dialect::for_language(lang, true, true);
                for dialect in dialects {
                    println!("  - {} ({})", dialect.name(), dialect.id());
                }
                println!();
            }
        }
        Commands::Update {
            url,
            api_key,
            dialect_from,
            dialect_to,
        } => {
            println!("Retrieving points to update");
            let qdrant = get_qdrant_service(url, api_key).await?;

            qdrant.update_points(&dialect_from, &dialect_to).await?;
        }
        Commands::Status { url, api_key } => {
            println!("🔍 Checking Qdrant Status");
            println!("{}\n", "=".repeat(50));

            // Connect to Qdrant
            let qdrant = get_qdrant_service(url, api_key).await?;

            // Get detailed status
            qdrant.get_detailed_status().await?;
        }
        Commands::ResetCollection { url, api_key, force } => {
             let qdrant = get_qdrant_service(url, api_key).await?;
             
             if !force {
                 use std::io::Write;
                 print!("⚠️  WARNING: This will DELETE existing collection 'dialect_documents_v2'. Type 'yes' to confirm: ");
                 std::io::stdout().flush()?;
                 let mut input = String::new();
                 std::io::stdin().read_line(&mut input)?;
                 if input.trim() != "yes" {
                     println!("Aborted.");
                     return Ok(());
                 }
             }
             
             qdrant.delete_collection().await?;
             println!("✅ Collection reset. It will be re-created with correct schema on next upload.");
        }
    }

    Ok(())
}

/// Parse dialect string - ONLY accepts canonical serde ID format
fn parse_dialect(_language: &str, dialect_name: &str) -> Result<Dialect> {
    // Use ONLY the canonical serde ID format parsing
    dialect_name
        .parse::<Dialect>()
        .map_err(|e| anyhow::anyhow!("Invalid dialect: {}. {}", dialect_name, e))
}

/// Load DialectDocuments from JSONL file
#[cfg(test)]
fn load_documents_from_jsonl(path: &str) -> Result<Vec<DialectDocument>> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read JSONL file: {}", path))?;

    let mut documents = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let doc: DialectDocument = serde_json::from_str(line)
            .context(format!("Failed to parse JSON at line {}", line_num + 1))?;

        documents.push(doc);
    }

    Ok(documents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{Dialect, Formality};
    use tempfile::tempdir;

    #[test]
    fn test_parse_dialect_valid() {
        // Test valid dialect IDs
        assert_eq!(
            parse_dialect("arabic", "arabic_egyptian").unwrap(),
            Dialect::ArabicEgyptian
        );
        assert_eq!(
            parse_dialect("spanish", "spanish_mexican").unwrap(),
            Dialect::SpanishMexican
        );
        assert_eq!(
            parse_dialect("french", "french_quebecois").unwrap(),
            Dialect::FrenchQuebecois
        );
    }

    #[test]
    fn test_parse_dialect_invalid() {
        // Test invalid dialect IDs
        let result = parse_dialect("invalid", "nonexistent_dialect");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Invalid dialect"));
        assert!(error.contains("nonexistent_dialect"));
    }

    #[test]
    fn test_parse_dialect_case_sensitive() {
        // Dialect parsing should be case sensitive and exact
        let result = parse_dialect("arabic", "Arabic_Egyptian");
        assert!(result.is_err());

        let result = parse_dialect("arabic", "ARABIC_EGYPTIAN");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_documents_from_jsonl_valid() {
        let temp_dir = tempdir().unwrap();
        let jsonl_path = temp_dir.path().join("test.jsonl");

        // Create test JSONL content
        let doc1 = DialectDocument::new(
            "Test content 1".to_string(),
            Dialect::ArabicEgyptian,
            Some(Formality::Formal),
        );
        let doc2 =
            DialectDocument::new("Test content 2".to_string(), Dialect::SpanishMexican, None);

        let json1 = serde_json::to_string(&doc1).unwrap();
        let json2 = serde_json::to_string(&doc2).unwrap();
        let content = format!("{}\n{}\n", json1, json2);

        fs::write(&jsonl_path, content).unwrap();

        // Test loading
        let docs = load_documents_from_jsonl(jsonl_path.to_str().unwrap()).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].content, "Test content 1");
        assert_eq!(docs[0].dialect, Dialect::ArabicEgyptian);
        assert_eq!(docs[1].content, "Test content 2");
        assert_eq!(docs[1].dialect, Dialect::SpanishMexican);
    }

    #[test]
    fn test_load_documents_from_jsonl_with_empty_lines() {
        let temp_dir = tempdir().unwrap();
        let jsonl_path = temp_dir.path().join("test.jsonl");

        let doc = DialectDocument::new("Test content".to_string(), Dialect::ArabicEgyptian, None);
        let json = serde_json::to_string(&doc).unwrap();

        // Include empty lines and whitespace
        let content = format!("\n{}\n\n  \n{}\n\n", json, json);
        fs::write(&jsonl_path, content).unwrap();

        let docs = load_documents_from_jsonl(jsonl_path.to_str().unwrap()).unwrap();
        assert_eq!(docs.len(), 2); // Empty lines should be skipped
    }

    #[test]
    fn test_load_documents_from_jsonl_empty_file() {
        let temp_dir = tempdir().unwrap();
        let jsonl_path = temp_dir.path().join("empty.jsonl");

        fs::write(&jsonl_path, "").unwrap();

        let docs = load_documents_from_jsonl(jsonl_path.to_str().unwrap()).unwrap();
        assert!(docs.is_empty());
    }

    #[test]
    fn test_load_documents_from_jsonl_file_not_found() {
        let result = load_documents_from_jsonl("/nonexistent/path.jsonl");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Failed to read JSONL file"));
    }

    #[test]
    fn test_load_documents_from_jsonl_invalid_json() {
        let temp_dir = tempdir().unwrap();
        let jsonl_path = temp_dir.path().join("invalid.jsonl");

        let doc = DialectDocument::new("Valid content".to_string(), Dialect::ArabicEgyptian, None);
        let valid_json = serde_json::to_string(&doc).unwrap();

        // Mix valid and invalid JSON
        let content = format!("{}\n{{\"broken json\"\n{}", valid_json, valid_json);
        fs::write(&jsonl_path, content).unwrap();

        let result = load_documents_from_jsonl(jsonl_path.to_str().unwrap());
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Failed to parse JSON at line 2"));
    }

    #[test]
    fn test_load_documents_from_jsonl_trailing_newline() {
        let temp_dir = tempdir().unwrap();
        let jsonl_path = temp_dir.path().join("test.jsonl");

        let doc = DialectDocument::new("Test content".to_string(), Dialect::ArabicEgyptian, None);
        let json = serde_json::to_string(&doc).unwrap();

        // Test with trailing newline
        fs::write(&jsonl_path, format!("{}\n", json)).unwrap();

        let docs = load_documents_from_jsonl(jsonl_path.to_str().unwrap()).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].content, "Test content");
    }
}
