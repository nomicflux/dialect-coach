mod chunking;
mod embeddings;
mod loaders;
mod processor;

use anyhow::Result;
use clap::{Parser, Subcommand};
use dialect_coach_shared::{Dialect, Language};

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

    /// List available dialects
    List,
}

fn main() -> Result<()> {
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
            processor::process_corpus(
                &input,
                &output,
                dialect_enum,
                chunk_size,
                overlap,
            )?;

            println!("\n✓ Processing completed successfully");
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
        _ => anyhow::bail!("Unknown language: {}. Use 'list' command to see available languages", language),
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
