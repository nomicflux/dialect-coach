mod chunking;
mod embeddings;
mod loaders;
mod processor;
mod qdrant;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialect_coach_shared::{Dialect, DialectDocument, Language};
use std::fs;

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

        /// Overlap between chunks in characters
        #[arg(long, default_value = "50")]
        overlap: usize,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Process {
            language,
            dialect,
            input,
            output,
            chunk_size,
            overlap,
        } => {
            println!("Processing corpus:");
            println!("  Language: {}", language);
            println!("  Dialect: {}", dialect);
            println!("  Input: {}", input);
            println!("  Output: {}", output);
            println!("  Chunk size: {}", chunk_size);
            println!("  Overlap: {}\n", overlap);

            // Parse dialect
            let dialect_enum = parse_dialect(&language, &dialect)?;

            // Process the corpus
            processor::process_corpus(&input, &output, dialect_enum, chunk_size, overlap)?;

            println!("\n✓ Processing completed successfully");
        }
        Commands::Upload {
            input,
            url,
            api_key,
        } => {
            // Get URL from argument or environment
            let qdrant_url = url.or_else(|| std::env::var("QDRANT_URL").ok()).context(
                "QDRANT_URL must be provided via --url flag or QDRANT_URL environment variable",
            )?;

            // Get API key from argument or environment
            let qdrant_api_key = api_key.or_else(|| std::env::var("QDRANT_API_KEY").ok());

            println!("Uploading documents to Qdrant:");
            println!("  Input: {}", input);
            println!("  Qdrant URL: {}\n", qdrant_url);

            // Load documents from JSONL file
            println!("Loading documents from {}...", input);
            let documents = load_documents_from_jsonl(&input)?;
            println!("Loaded {} documents\n", documents.len());

            // Connect to Qdrant and upload
            let qdrant = if let Some(key) = qdrant_api_key {
                qdrant::QdrantService::new_with_api_key(&qdrant_url, &key).await?
            } else {
                qdrant::QdrantService::new(&qdrant_url).await?
            };

            println!("Uploading to Qdrant...");
            qdrant.upload_documents(&documents).await?;

            qdrant.get_collection_info().await?;

            println!("\n✓ Upload completed successfully");
        }
        Commands::List => {
            println!("Available languages and dialects:\n");

            for lang in Language::all() {
                println!("{}:", lang.name());
                let dialects = Dialect::for_language(lang);
                for dialect in dialects {
                    println!("  - {} ({})", dialect.name(), dialect.bcp47_tag());
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Parse language and dialect strings into Dialect enum
fn parse_dialect(language: &str, dialect_name: &str) -> Result<Dialect> {
    // Parse language
    let lang = match language.to_lowercase().as_str() {
        "spanish" | "es" => Language::Spanish,
        "arabic" | "ar" => Language::Arabic,
        "french" | "fr" => Language::French,
        _ => anyhow::bail!(
            "Unknown language: {}. Use 'list' command to see available languages",
            language
        ),
    };

    // Find matching dialect
    let dialects = Dialect::for_language(lang);
    let dialect_lower = dialect_name.to_lowercase();

    for dialect in dialects {
        let name_lower = dialect.name().to_lowercase();
        if name_lower.contains(&dialect_lower) || dialect_lower.contains(&name_lower) {
            return Ok(dialect);
        }
    }

    anyhow::bail!(
        "Unknown dialect '{}' for language '{}'. Use 'list' command to see available dialects",
        dialect_name,
        language
    )
}

/// Load DialectDocuments from JSONL file
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
