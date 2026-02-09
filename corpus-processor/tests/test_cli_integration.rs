use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

// Note: Tests for "missing QDRANT_URL" scenarios were removed because
// config.yaml (now required for qdrant commands) always provides a URL fallback.

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Process dialect corpus data for RAG",
        ))
        .stdout(predicate::str::contains("process"))
        .stdout(predicate::str::contains("upload"));
}

#[test]
fn test_cli_list_dialects() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Available languages and dialects"))
        .stdout(predicate::str::contains("Spanish"))
        .stdout(predicate::str::contains("Arabic"))
        .stdout(predicate::str::contains("French"));
}

#[test]
fn test_cli_process_missing_required_args() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.arg("process");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_cli_process_missing_language() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.args(["process", "--dialect", "egyptian", "--input", "./test"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("language"));
}

#[test]
fn test_cli_process_missing_dialect() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.args(["process", "--language", "arabic", "--input", "./test"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("dialect"));
}

#[test]
fn test_cli_process_missing_input() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.args(["process", "--language", "arabic", "--dialect", "egyptian"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("input"));
}

#[test]
fn test_cli_process_invalid_dialect() {
    let temp_dir = tempdir().unwrap();
    let input_file = temp_dir.path().join("test.txt");
    fs::write(&input_file, "test content").unwrap();

    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.args([
        "process",
        "--language",
        "arabic",
        "--dialect",
        "invalid_dialect",
        "--input",
        input_file.to_str().unwrap(),
    ]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid dialect"));
}

#[test]
fn test_cli_process_nonexistent_input() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.args([
        "process",
        "--language",
        "arabic",
        "--dialect",
        "arabic_egyptian",
        "--input",
        "./nonexistent_path",
    ]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_cli_upload_missing_input() {
    let mut cmd = Command::cargo_bin("corpus-processor").unwrap();
    cmd.arg("upload");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("input"));
}

