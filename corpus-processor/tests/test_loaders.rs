use corpus_processor::loaders::load_corpus;
use dialect_coach_shared::Dialect;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_load_nonexistent_path() {
    let result = load_corpus("./nonexistent/path", Dialect::ArabicEgyptian);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn test_load_empty_txt_file() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("empty.txt");
    fs::write(&file_path, "").unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert!(docs.is_empty());
}

#[test]
fn test_load_txt_file() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let content = "This is test content in Arabic: مرحبا";
    fs::write(&file_path, content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].content, content);
    assert_eq!(docs[0].dialect, Dialect::ArabicEgyptian);
    assert!(docs[0].formality.is_none());
}

#[test]
fn test_load_txt_file_multiline() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("multiline.txt");
    // Multiple lines - each should be a separate document
    let content = "First line in Arabic: مرحبا\nSecond line: كيف حالك\nThird line: شكرا";
    fs::write(&file_path, content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicLevantine).unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].content, "First line in Arabic: مرحبا");
    assert_eq!(docs[1].content, "Second line: كيف حالك");
    assert_eq!(docs[2].content, "Third line: شكرا");
    assert_eq!(docs[0].dialect, Dialect::ArabicLevantine);
}

#[test]
fn test_load_csv_with_headers() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.csv");
    let csv_content = "content,formality\n\"Hello world\",formal\n\"مرحبا\",casual";
    fs::write(&file_path, csv_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2);

    assert_eq!(docs[0].content, "Hello world");
    assert!(docs[0].formality.is_some());

    assert_eq!(docs[1].content, "مرحبا");
    assert!(docs[1].formality.is_some());
}

#[test]
fn test_load_csv_missing_content_column() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("bad.csv");
    let csv_content = "text_column,other\n\"Hello\",world";
    fs::write(&file_path, csv_content).unwrap();

    let result = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("content"));
}

#[test]
fn test_load_csv_with_text_column() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.csv");
    let csv_content = "text,other\n\"Hello world\",data\n\"مرحبا\",more";
    fs::write(&file_path, csv_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].content, "Hello world");
    assert_eq!(docs[1].content, "مرحبا");
}

#[test]
fn test_load_tsv_file() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.tsv");
    let tsv_content = "file001\tHello world\nfile002\tمرحبا بك";
    fs::write(&file_path, tsv_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].content, "Hello world");
    assert_eq!(docs[1].content, "مرحبا بك");
    assert!(docs[0].formality.is_none());
}

#[test]
fn test_load_json_array() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let json_content = r#"[
        {"content": "Hello world", "formality": "formal"},
        {"text": "مرحبا", "formality": "casual"}
    ]"#;
    fs::write(&file_path, json_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].content, "Hello world");
    assert_eq!(docs[1].content, "مرحبا");
}

#[test]
fn test_load_json_single_object() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let json_content = r#"{"content": "Single document", "formality": "formal"}"#;
    fs::write(&file_path, json_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].content, "Single document");
}

#[test]
fn test_load_json_missing_content() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("bad.json");
    let json_content = r#"{"other_field": "value"}"#;
    fs::write(&file_path, json_content).unwrap();

    let result = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian);
    assert!(result.is_err());
}

#[test]
fn test_load_directory() {
    let temp_dir = tempdir().unwrap();

    // Create multiple files
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");
    fs::write(&file1, "Content 1").unwrap();
    fs::write(&file2, "Content 2").unwrap();

    let docs = load_corpus(temp_dir.path().to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2);

    // Content order might vary, so check both are present
    let contents: Vec<&str> = docs.iter().map(|d| d.content.as_str()).collect();
    assert!(contents.contains(&"Content 1"));
    assert!(contents.contains(&"Content 2"));
}

#[test]
fn test_unsupported_file_format() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.xyz");
    fs::write(&file_path, "content").unwrap();

    let result = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unsupported file format")
    );
}

#[test]
fn test_skip_empty_content() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.csv");
    let csv_content = "content\n\"Valid content\"\n\"\"\n\"   \"\n\"Another valid\"";
    fs::write(&file_path, csv_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2); // Empty and whitespace-only should be skipped
    assert_eq!(docs[0].content, "Valid content");
    assert_eq!(docs[1].content, "Another valid");
}

#[test]
fn test_load_csv_invalid_formality() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.csv");
    let csv_content = "content,formality\n\"Hello\",invalid_formality\n\"World\",casual";
    fs::write(&file_path, csv_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 2);

    // Invalid formality should become None
    assert!(docs[0].formality.is_none());
    // Valid formality should be parsed
    assert!(docs[1].formality.is_some());
}

#[test]
fn test_load_tsv_short_rows() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.tsv");
    // Inconsistent column count should cause CSV error
    let tsv_content = "onlyonecol\nfile002\tHello world";
    fs::write(&file_path, tsv_content).unwrap();

    let result = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian);
    // Should fail due to inconsistent column count
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("CSV error"));
}

#[test]
fn test_load_json_array_non_objects() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("bad.json");
    let json_content = "[1, 2, 3]"; // Not objects
    fs::write(&file_path, json_content).unwrap();

    let result = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian);
    // Numbers in array should succeed but produce no documents
    assert!(result.is_ok());
    let docs = result.unwrap();
    assert!(docs.is_empty()); // No valid content extracted
}

#[test]
fn test_load_json_malformed() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("bad.json");
    let json_content = "{broken json}";
    fs::write(&file_path, json_content).unwrap();

    let result = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian);
    assert!(result.is_err());
}

#[test]
fn test_load_directory_with_unsupported_files() {
    let temp_dir = tempdir().unwrap();

    // Create mix of supported and unsupported files
    let good_file = temp_dir.path().join("good.txt");
    let bad_file = temp_dir.path().join("bad.xyz");
    fs::write(&good_file, "Good content").unwrap();
    fs::write(&bad_file, "Bad content").unwrap();

    // Should load only the supported file, warn about unsupported
    let docs = load_corpus(temp_dir.path().to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].content, "Good content");
}

#[test]
fn test_load_directory_empty() {
    let temp_dir = tempdir().unwrap();

    // Empty directory should return empty vec, not error
    let docs = load_corpus(temp_dir.path().to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert!(docs.is_empty());
}

#[test]
fn test_parse_formality_variants() {
    // Test all formality parsing edge cases
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.csv");
    let csv_content = "content,formality\n\
        \"Test1\",formal\n\
        \"Test2\",Formal\n\
        \"Test3\",CASUAL\n\
        \"Test4\",informal\n\
        \"Test5\",slang\n\
        \"Test6\",SLANG\n\
        \"Test7\",unknown";
    fs::write(&file_path, csv_content).unwrap();

    let docs = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian).unwrap();
    assert_eq!(docs.len(), 7);

    // Test case-insensitive parsing
    use dialect_coach_shared::Formality;
    assert_eq!(docs[0].formality, Some(Formality::Formal));
    assert_eq!(docs[1].formality, Some(Formality::Formal));
    assert_eq!(docs[2].formality, Some(Formality::Casual));
    assert_eq!(docs[3].formality, Some(Formality::Casual)); // informal -> casual
    assert_eq!(docs[4].formality, Some(Formality::Slang));
    assert_eq!(docs[5].formality, Some(Formality::Slang));
    assert!(docs[6].formality.is_none()); // unknown
}

#[test]
fn test_load_csv_malformed() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("bad.csv");
    // Create truly malformed CSV - use incomplete escape
    let csv_content = "content\nvalue1\nvalue2\n,,,invalid";
    fs::write(&file_path, csv_content).unwrap();

    // Actually this might not error - CSV is quite forgiving
    // Let's test a case that definitely errors - invalid UTF-8
    let file_path2 = temp_dir.path().join("bad2.csv");
    let invalid_utf8 = vec![0xff, 0xfe]; // Invalid UTF-8 bytes
    std::fs::write(&file_path2, invalid_utf8).unwrap();

    let result = load_corpus(file_path2.to_str().unwrap(), Dialect::ArabicEgyptian);
    assert!(result.is_err());
}
