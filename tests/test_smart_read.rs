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

use codeaware_mcp::tools::smart_read::SmartReadError;

fn read_input(path: &str) -> SmartReadInput {
    SmartReadInput {
        path: path.to_string(),
        mode: ReadMode::Full,
        focus: None,
        lines: None,
        scope: None,
    }
}

#[test]
fn test_relative_path_read_from_project_root() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "fn a() {}\n").unwrap();
    let result = smart_read(&read_input("src/lib.rs"), dir.path()).unwrap();
    assert_eq!(result.loc, 1);
}

#[test]
fn test_traversal_outside_root_rejected() {
    let base = TempDir::new().unwrap();
    let project = base.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(base.path().join("outside.rs"), "fn x() {}\n").unwrap();
    let result = smart_read(&read_input("../outside.rs"), &project);
    assert!(result.is_err(), "traversal must be rejected");
    let abs = base.path().join("outside.rs");
    let result = smart_read(&read_input(&abs.to_string_lossy()), &project);
    assert!(result.is_err(), "absolute path outside root must be rejected");
}

#[test]
fn test_denied_read_patterns_enforced() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".env"), "SECRET=1\n").unwrap();
    fs::create_dir_all(dir.path().join("target/debug")).unwrap();
    fs::write(dir.path().join("target/debug/app.rs"), "fn x() {}\n").unwrap();
    let env = smart_read(&read_input(&dir.path().join(".env").to_string_lossy()), dir.path());
    assert!(matches!(env, Err(SmartReadError::Denied(_))), "got {env:?}");
    let target = smart_read(&read_input("target/debug/app.rs"), dir.path());
    assert!(matches!(target, Err(SmartReadError::Denied(_))), "got {target:?}");
}

#[test]
fn test_symlink_escape_for_missing_child_rejected() {
    let base = TempDir::new().unwrap();
    let project = base.path().join("project");
    let outside = base.path().join("outside");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, project.join("link")).unwrap();
    let result = smart_read(&read_input("link/new.conf"), &project);
    assert!(result.is_err());
}

#[test]
fn test_read_over_size_limit_is_refused() {
    use codeaware_mcp::tools::smart_read::{SmartReadError, MAX_READ_BYTES};
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("huge.txt");
    let f = fs::File::create(&file).unwrap();
    f.set_len(MAX_READ_BYTES + 1).unwrap();
    drop(f);

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Full,
        focus: None,
        lines: None,
        scope: None,
    };
    let err = smart_read(&input, dir.path()).expect_err("oversized file must be refused");
    assert!(matches!(err, SmartReadError::TooLarge { .. }), "got {err}");
    assert!(err.to_string().contains("focused"), "message should suggest a focused read: {err}");
}

#[test]
fn test_read_at_size_limit_is_not_refused_for_size() {
    use codeaware_mcp::tools::smart_read::{SmartReadError, MAX_READ_BYTES};
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("edge.txt");
    let f = fs::File::create(&file).unwrap();
    f.set_len(MAX_READ_BYTES).unwrap();
    drop(f);

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Full,
        focus: None,
        lines: None,
        scope: None,
    };
    // All-zero content is detected as binary, which proves the size check let it through.
    let err = smart_read(&input, dir.path()).expect_err("zero-filled file is binary");
    assert!(!matches!(err, SmartReadError::TooLarge { .. }), "limit itself must be allowed: {err}");
}
