use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Formality};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Load corpus documents from a file or directory
pub fn load_corpus(input_path: &str, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let path = Path::new(input_path);

    if !path.exists() {
        anyhow::bail!("Input path does not exist: {}", input_path);
    }

    if path.is_file() {
        load_single_file(path, dialect)
    } else {
        load_directory(path, dialect)
    }
}

/// Load a single corpus file
fn load_single_file(path: &Path, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "txt" => load_text_file(path, dialect),
        "csv" => load_csv_file(path, dialect),
        "tsv" => load_tsv_file(path, dialect),
        "json" => load_json_file(path, dialect),
        _ => {
            anyhow::bail!("Unsupported file format: {}", extension);
        }
    }
}

/// Load all supported files from a directory
fn load_directory(path: &Path, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let mut documents = Vec::new();

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            match load_single_file(entry.path(), dialect.clone()) {
                Ok(docs) => {
                    println!(
                        "  Loaded {} documents from {}",
                        docs.len(),
                        entry.path().display()
                    );
                    documents.extend(docs);
                }
                Err(e) => {
                    eprintln!(
                        "  Warning: Failed to load {}: {}",
                        entry.path().display(),
                        e
                    );
                }
            }
        }
    }

    Ok(documents)
}

/// Load plain text file - one document per file
fn load_text_file(path: &Path, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read file: {}", path.display()))?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let doc = DialectDocument::new(content.trim().to_string(), dialect, None);
    Ok(vec![doc])
}

/// Load CSV file with columns: content, formality (optional)
fn load_csv_file(path: &Path, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let mut documents = Vec::new();
    let mut reader = csv::Reader::from_path(path)
        .context(format!("Failed to read CSV file: {}", path.display()))?;

    // Check headers
    let headers = reader.headers()?.clone();
    let content_col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("content") || h.eq_ignore_ascii_case("text"))
        .context("CSV must have 'content' or 'text' column")?;

    let formality_col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("formality"));

    for result in reader.records() {
        let record = result?;

        if let Some(content) = record.get(content_col) {
            if content.trim().is_empty() {
                continue;
            }

            let formality = formality_col
                .and_then(|col| record.get(col))
                .and_then(parse_formality);

            let doc = DialectDocument::new(content.trim().to_string(), dialect.clone(), formality);
            documents.push(doc);
        }
    }

    Ok(documents)
}

/// Load TSV file without headers (OpenSLR format: fileID<tab>transcription)
fn load_tsv_file(path: &Path, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let mut documents = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(path)
        .context(format!("Failed to read TSV file: {}", path.display()))?;

    for result in reader.records() {
        let record = result?;

        // Expect format: fileID<tab>transcription
        if record.len() >= 2 {
            let content = &record[1];

            if content.trim().is_empty() {
                continue;
            }

            let doc = DialectDocument::new(content.trim().to_string(), dialect.clone(), None);
            documents.push(doc);
        }
    }

    Ok(documents)
}

/// Load JSON file - array of objects with content and optional formality
fn load_json_file(path: &Path, dialect: Dialect) -> Result<Vec<DialectDocument>> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read JSON file: {}", path.display()))?;

    let json: serde_json::Value = serde_json::from_str(&content).context("Failed to parse JSON")?;

    let mut documents = Vec::new();

    // Handle both single object and array
    let items = match json {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::Object(_) => vec![json],
        _ => anyhow::bail!("JSON must be an object or array"),
    };

    for item in items {
        if let Some(obj) = item.as_object() {
            // Look for content field
            let content = obj
                .get("content")
                .or_else(|| obj.get("text"))
                .and_then(|v| v.as_str())
                .context("JSON object must have 'content' or 'text' field")?;

            if content.trim().is_empty() {
                continue;
            }

            // Parse formality if present
            let formality = obj
                .get("formality")
                .and_then(|v| v.as_str())
                .and_then(parse_formality);

            let doc = DialectDocument::new(content.trim().to_string(), dialect.clone(), formality);
            documents.push(doc);
        }
    }

    Ok(documents)
}

/// Parse formality string into enum
fn parse_formality(s: &str) -> Option<Formality> {
    match s.trim().to_lowercase().as_str() {
        "formal" => Some(Formality::Formal),
        "casual" | "informal" => Some(Formality::Casual),
        "slang" => Some(Formality::Slang),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_formality() {
        assert!(matches!(parse_formality("formal"), Some(Formality::Formal)));
        assert!(matches!(parse_formality("Casual"), Some(Formality::Casual)));
        assert!(matches!(parse_formality("SLANG"), Some(Formality::Slang)));
        assert!(parse_formality("unknown").is_none());
    }
}
