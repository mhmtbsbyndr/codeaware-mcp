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

#[test]
fn test_empty_old_text_is_rejected_without_hanging() {
    // An empty `old` used to loop forever in count_occurrences. Run the call on a
    // worker thread so a regression fails the test instead of hanging the suite.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn hello() {}\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: String::new(), new: "x".into() }]),
        ..Default::default()
    };
    let root = dir.path().to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(smart_edit(&input, &root).is_err());
    });
    let is_err = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("smart_edit with empty `old` must not hang");
    assert!(is_err, "empty `old` must be rejected with an error");
    assert_eq!(fs::read_to_string(&file).unwrap(), "fn hello() {}\n");
}

#[test]
fn test_lines_strategy_preserves_crlf() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "line1\r\nline2\r\nline3\r\nline4\r\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "lines".into(),
        line_range: Some("2-3".into()),
        new_content: Some("replaced2\nreplaced3".into()),
        ..Default::default()
    };
    smart_edit(&input, dir.path()).unwrap();
    let bytes = fs::read(&file).unwrap();
    assert_eq!(bytes, b"line1\r\nreplaced2\r\nreplaced3\r\nline4\r\n");
}

#[test]
fn test_text_strategy_matches_multiline_text_in_crlf_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn a() {\r\n    1\r\n}\r\n").unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: "    1\n}".into(), new: "    2\n}".into() }]),
        ..Default::default()
    };
    smart_edit(&input, dir.path()).unwrap();
    assert_eq!(fs::read(&file).unwrap(), b"fn a() {\r\n    2\r\n}\r\n");
}

#[test]
fn test_non_utf8_file_is_refused_and_untouched() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.txt");
    let original: Vec<u8> = vec![b'a', b' ', b'=', b' ', 0xff, 0xfe, b'\n'];
    fs::write(&file, &original).unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: "a".into(), new: "b".into() }]),
        ..Default::default()
    };
    assert!(smart_edit(&input, dir.path()).is_err());
    assert_eq!(fs::read(&file).unwrap(), original, "non-UTF-8 bytes must not be rewritten");
}

#[test]
fn test_edit_denied_lock_file_leaves_file_unchanged() {
    use codeaware_mcp::tools::smart_edit::SmartEditError;
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("Cargo.lock");
    let original = "# lock\nversion = 3\n";
    fs::write(&file, original).unwrap();
    let input = SmartEditInput {
        path: file.to_string_lossy().to_string(),
        strategy: "text".into(),
        edits: Some(vec![EditPair { old: "version = 3".into(), new: "version = 4".into() }]),
        ..Default::default()
    };
    let err = smart_edit(&input, dir.path()).expect_err("Cargo.lock must be refused");
    assert!(matches!(err, SmartEditError::Denied(_)), "got {err}");
    assert_eq!(fs::read_to_string(&file).unwrap(), original);
}
