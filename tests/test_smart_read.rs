use codeaware_mcp::tools::smart_read::{smart_read, SmartReadInput, ReadMode};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_small_file_returns_full() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("small.rs");
    fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Auto,
        focus: None,
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "full");
    assert!(result.content.is_some());
    assert_eq!(result.loc, 3);
    assert!(!result.stale);
}

#[test]
fn test_large_file_returns_skeleton() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("large.rs");
    let content: String = (0..200).map(|i| format!("fn func_{i}() {{}}\n")).collect();
    fs::write(&file, &content).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Auto,
        focus: None,
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "skeleton");
    assert!(result.content.is_some());
    let content_lines = result.content.unwrap().lines().count();
    assert!(content_lines < 200);
}

#[test]
fn test_explicit_full_mode() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("file.rs");
    let content: String = (0..200).map(|i| format!("fn func_{i}() {{}}\n")).collect();
    fs::write(&file, &content).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Full,
        focus: None,
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "full");
    assert_eq!(result.loc, 200);
}

#[test]
fn test_line_range_read() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("file.rs");
    let content: String = (1..=100).map(|i| format!("line {i}\n")).collect();
    fs::write(&file, &content).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Auto,
        focus: None,
        lines: Some("10-20".into()),
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "focused");
    let content = result.content.unwrap();
    assert!(content.contains("line 10"));
    assert!(content.contains("line 20"));
    assert!(!content.contains("line 9\n"));
    assert!(!content.contains("line 21"));
}

#[test]
fn test_focused_read_with_search() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("file.rs");
    let mut content = String::new();
    for i in 1..=50 {
        content.push_str(&format!("fn unrelated_{i}() {{}}\n"));
    }
    content.push_str("pub fn verify_token(token: &str) -> bool {\n");
    content.push_str("    token.len() > 10\n");
    content.push_str("}\n");
    for i in 1..=50 {
        content.push_str(&format!("fn other_{i}() {{}}\n"));
    }
    fs::write(&file, &content).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Auto,
        focus: Some("verify_token".into()),
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "focused");
    let content = result.content.unwrap();
    assert!(content.contains("verify_token"));
}

#[test]
fn test_binary_file_returns_error() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("binary.wasm");
    fs::write(&file, &[0u8, 1, 2, 3, 0xFF, 0xFE]).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Auto,
        focus: None,
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path());
    assert!(result.is_err());
}
