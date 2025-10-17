# Clap Tutorial: Command-Line Argument Parsing

## Overview

**clap** is a full-featured, fast command-line argument parser for Rust. It supports derive macros for declarative CLI definition, subcommands, validation, and auto-generated help.

**Version used in dialect-coach:** `4.5` with derive feature

**Why we use it:** Provides user-friendly CLIs with minimal code, auto-generates help text, supports complex argument patterns.

## Dependencies

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

## Core Concepts

### 1. Derive-Based CLIs

Use attributes to define CLI:

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "my-app")]
#[command(about = "Does awesome things", long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

fn main() {
    let cli = Cli::parse();
    println!("Verbose: {}", cli.verbose);
    println!("Config: {}", cli.config);
}
```

### 2. Subcommands

Git-like command structure:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add { name: String },
    Remove { name: String },
    List,
}
```

### 3. Arguments

Different argument types:

```rust
#[derive(Parser)]
struct Cli {
    // Positional argument (required)
    input: String,

    // Optional flag
    // short = -v, long = --verbose
    #[arg(short, long)]
    verbose: bool,

    // Option with value
    // short = -o, long = --output
    #[arg(short, long)]
    output: Option<String>,

    // With default
    // short = -c, long = --count
    #[arg(short, long, default_value = "5")]
    count: usize,
}
```

**Short vs Long Flags:**
- **`short`** - Single character flag, prefixed with `-` (e.g., `-v`, `-o`)
- **`long`** - Full word flag, prefixed with `--` (e.g., `--verbose`, `--output`)
- Both can be used simultaneously: `myapp -v` or `myapp --verbose`
- `short` automatically uses first letter of field name unless specified: `#[arg(short = 'x')]`

## Step-by-Step: Building a File Manager CLI

### Step 1: Basic CLI

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "fileman")]
#[command(version = "1.0")]
#[command(about = "A simple file manager", long_about = None)]
struct Cli {
    /// Input file path
    file: String,

    /// Be verbose
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Cli::parse();

    if args.verbose {
        println!("Processing file: {}", args.file);
    }

    // Process file...
}
```

**Doc Comments → Help Text:** The `///` doc comments above each field automatically become help text in the `--help` output. Clap transforms:
- `/// Input file path` → help text for the `file` argument
- `/// Be verbose` → help text for `--verbose` flag

This keeps documentation and CLI help in sync!

Usage:
```bash
$ fileman input.txt
$ fileman input.txt --verbose
$ fileman --help
# Output shows:
# Arguments:
#   <FILE>  Input file path
#
# Options:
#   -v, --verbose  Be verbose
```

### Step 2: Adding Subcommands

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fileman")]
#[command(about = "File manager with subcommands")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy a file
    Copy {
        /// Source file
        source: String,

        /// Destination
        dest: String,

        /// Overwrite existing files
        #[arg(short, long)]
        force: bool,
    },

    /// Delete a file
    Delete {
        /// File to delete
        file: String,
    },

    /// List files
    List {
        /// Directory path
        #[arg(default_value = ".")]
        path: String,

        /// Show hidden files
        #[arg(short, long)]
        all: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Copy { source, dest, force } => {
            println!("Copying {} to {} (force: {})", source, dest, force);
            // Copy logic...
        }
        Commands::Delete { file } => {
            println!("Deleting {}", file);
            // Delete logic...
        }
        Commands::List { path, all } => {
            println!("Listing {} (all: {})", path, all);
            // List logic...
        }
    }
}
```

Usage:
```bash
$ fileman copy file.txt backup.txt
$ fileman copy file.txt backup.txt --force
$ fileman delete old.txt
$ fileman list
$ fileman list /tmp --all
```

### Step 3: Optional and Default Values

```rust
#[derive(Parser)]
struct Cli {
    /// Input file
    input: String,

    /// Output file (optional)
    #[arg(short, long)]
    output: Option<String>,

    /// Number of threads
    #[arg(short = 'j', long, default_value = "4")]
    threads: usize,

    /// Compression level (0-9)
    #[arg(short, long, default_value = "6")]
    level: u8,
}

fn main() {
    let args = Cli::parse();

    let output = args.output.unwrap_or_else(|| {
        format!("{}.out", args.input)
    });

    println!("Input: {}", args.input);
    println!("Output: {}", output);
    println!("Threads: {}", args.threads);
    println!("Level: {}", args.level);
}
```

### Step 4: Validation and Value Parsers

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Port number (1-65535)
    #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..=65535))]
    port: u16,

    /// Log level
    #[arg(long, value_parser = ["debug", "info", "warn", "error"])]
    log_level: String,

    /// Output format
    #[arg(short, long, value_enum)]
    format: OutputFormat,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Yaml,
    Toml,
}

fn main() {
    let args = Cli::parse();

    println!("Port: {}", args.port);
    println!("Log level: {}", args.log_level);
    println!("Format: {:?}", args.format);
}
```

## How dialect-coach Uses Clap

### 1. CLI Structure (corpus-processor/src/main.rs:12-66)

Main CLI with subcommands:

```rust
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

        /// Output directory
        #[arg(short, long, default_value = "output")]
        output: String,

        /// Maximum chunk size
        #[arg(long, default_value = "512")]
        chunk_size: usize,

        /// Overlap between chunks
        #[arg(long, default_value = "50")]
        overlap: usize,
    },

    /// Upload processed documents to Qdrant
    Upload {
        /// Path to processed documents (JSONL file)
        #[arg(short, long)]
        input: String,

        /// Qdrant server URL
        #[arg(short, long)]
        url: Option<String>,

        /// Qdrant API key
        #[arg(short = 'k', long)]
        api_key: Option<String>,
    },

    /// List available dialects
    List,
}
```

**Pattern:** Three subcommands for different operations, each with specific arguments.

### 2. Parsing and Dispatching (corpus-processor/src/main.rs:73-153)

Match on subcommands:

```rust
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

        let dialect_enum = parse_dialect(&language, &dialect)?;

        processor::process_corpus(
            &input,
            &output,
            dialect_enum,
            chunk_size,
            overlap,
        )?;

        println!("\n✓ Processing completed");
    }
    Commands::Upload { input, url, api_key } => {
        // Get from args or environment
        let qdrant_url = url
            .or_else(|| std::env::var("QDRANT_URL").ok())
            .context("QDRANT_URL required")?;

        let qdrant_api_key = api_key
            .or_else(|| std::env::var("QDRANT_API_KEY").ok());

        // Upload logic...
    }
    Commands::List => {
        println!("Available languages and dialects:\n");
        for lang in Language::all() {
            println!("{}:", lang.name());
            // List dialects...
        }
    }
}
```

**Pattern:** Extract values from commands, validate, execute operation.

### 3. Environment Fallback (corpus-processor/src/main.rs:108-114)

Combine args with environment:

```rust
Commands::Upload { input, url, api_key } => {
    // Try argument first, then environment variable
    let qdrant_url = url
        .or_else(|| std::env::var("QDRANT_URL").ok())
        .context("QDRANT_URL must be provided via --url or environment")?;

    let qdrant_api_key = api_key
        .or_else(|| std::env::var("QDRANT_API_KEY").ok());

    // Use values...
}
```

**Pattern:** Optional CLI args with environment variable fallback. Good for secrets.

### 4. Help Documentation (corpus-processor/src/main.rs:23-30)

Generated help text:

```rust
/// Load corpus files and generate embeddings
Process {
    /// Language to process
    #[arg(short, long)]
    language: String,

    /// Dialect to process
    #[arg(short, long)]
    dialect: String,

    // ...
}
```

**Pattern:** Doc comments become help text. Clap auto-generates usage.

Usage output:
```bash
$ corpus-processor process --help
Load corpus files and generate embeddings

Usage: corpus-processor process [OPTIONS] --language <LANGUAGE> --dialect <DIALECT> --input <INPUT>

Options:
  -l, --language <LANGUAGE>      Language to process
  -d, --dialect <DIALECT>        Dialect to process
  -i, --input <INPUT>            Input corpus file or directory
  -o, --output <OUTPUT>          Output directory [default: output]
      --chunk-size <CHUNK_SIZE>  Maximum chunk size [default: 512]
      --overlap <OVERLAP>        Overlap between chunks [default: 50]
  -h, --help                     Print help
```

## Common Patterns

### Pattern 1: Global Flags

```rust
#[derive(Parser)]
struct Cli {
    /// Global verbose flag
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}
```

### Pattern 2: Multiple Values

```rust
#[derive(Parser)]
struct Cli {
    /// Input files (multiple)
    #[arg(short, long, num_args = 1..)]
    files: Vec<String>,
}
```

### Pattern 3: Conflicting Arguments

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, conflicts_with = "json")]
    yaml: bool,

    #[arg(long, conflicts_with = "yaml")]
    json: bool,
}
```

## Best Practices from dialect-coach

1. **Descriptive help text** - Use doc comments for every argument
2. **Subcommands for operations** - Better UX than flags
3. **Default values** - Make CLIs ergonomic
4. **Environment fallback** - Especially for secrets
5. **Validation at parse time** - Use value_parser for ranges
6. **Short and long forms** - `-i` and `--input` for flexibility

## Troubleshooting

### Issue: "Required argument not provided"

**Cause:** Missing required argument

**Solution:** Make it `Option<T>` or provide default:

```rust
#[arg(short, long)]
optional: Option<String>,

#[arg(short, long, default_value = "default")]
with_default: String,
```

### Issue: Help text not showing

**Cause:** Missing doc comments

**Solution:** Add doc comments:

```rust
/// This is the help text
#[arg(short, long)]
my_arg: String,
```

### Issue: Subcommand not recognized

**Cause:** Typo or missing variant

**Solution:** Check enum matches command name:

```rust
#[derive(Subcommand)]
enum Commands {
    MyCommand { },  // Invoked as "my-command"
}
```

## Further Resources

- **Official Docs:** https://docs.rs/clap/latest/clap/
- **Derive Tutorial:** https://github.com/clap-rs/clap/tree/master/clap_derive
- **Examples:** https://github.com/clap-rs/clap/tree/master/examples
- **Book:** https://docs.rs/clap/latest/clap/_derive/index.html

## Summary

Clap provides command-line parsing:

- **Derive macros** for declarative CLIs
- **Subcommands** for complex CLIs
- **Auto-generated help** text
- **Validation** at parse time
- **Flexible arguments** (positional, optional, flags)
- **Environment integration** for secrets

The dialect-coach project demonstrates production CLI patterns: subcommands for different operations, environment fallback for configuration, comprehensive help text, and proper error handling.
