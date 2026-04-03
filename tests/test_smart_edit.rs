use codeaware_mcp::tools::smart_edit::{smart_edit, SmartEditInput, EditPair};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_single_text_replace() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn hello() -> i32 {\n    42\n}\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: "-> i32".into(), new: "-> u64".into() }]),
        ..Default::default()
    };
    let result = smart_edit(&input, dir.path()).unwrap();
    assert!(result.applied);
    assert_eq!(result.strategy_used, "text");
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("-> u64"));
    assert!(!content.contains("-> i32"));
}

#[test]
fn test_ambiguous_match_zero() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn hello() {}\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: "nonexistent text".into(), new: "replacement".into() }]),
        ..Default::default()
    };
    assert!(smart_edit(&input, dir.path()).is_err());
}

#[test]
fn test_ambiguous_match_multiple() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "let x = 1;\nlet y = 1;\nlet z = 1;\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: "= 1".into(), new: "= 2".into() }]),
        ..Default::default()
    };
    assert!(smart_edit(&input, dir.path()).is_err());
}

#[test]
fn test_sequential_edits() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn foo() -> i32 { 1 }\nfn bar() -> i32 { 2 }\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![
            EditPair { old: "fn foo() -> i32 { 1 }".into(), new: "fn foo() -> u64 { 1 }".into() },
            EditPair { old: "fn bar() -> i32 { 2 }".into(), new: "fn bar() -> u64 { 2 }".into() },
        ]),
        ..Default::default()
    };
    let result = smart_edit(&input, dir.path()).unwrap();
    assert!(result.applied);
    assert_eq!(result.edits_applied.len(), 2);
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("fn foo() -> u64"));
    assert!(content.contains("fn bar() -> u64"));
}

#[test]
fn test_hash_mismatch_blocks_edit() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn hello() {}\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        expected_hash: Some("wrong_hash".into()),
        edits: Some(vec![EditPair { old: "fn hello".into(), new: "fn world".into() }]),
        ..Default::default()
    };
    assert!(smart_edit(&input, dir.path()).is_err());
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("fn hello"));
}

#[test]
fn test_dry_run_does_not_modify() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    let original = "fn hello() -> i32 { 42 }\n";
    fs::write(&file, original).unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        dry_run: true,
        edits: Some(vec![EditPair { old: "-> i32".into(), new: "-> u64".into() }]),
        ..Default::default()
    };
    let result = smart_edit(&input, dir.path()).unwrap();
    assert!(!result.applied);
    assert!(result.dry_run);
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content, original);
}

#[test]
fn test_line_range_edit() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "line1\nline2\nline3\nline4\nline5\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "lines".into(),
        line_range: Some("2-3".into()),
        new_content: Some("replaced2\nreplaced3".into()),
        ..Default::default()
    };
    let result = smart_edit(&input, dir.path()).unwrap();
    assert!(result.applied);
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("line1\nreplaced2\nreplaced3\nline4"));
}
