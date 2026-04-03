use codeaware_mcp::security::path_resolver::{resolve_path, PathError};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_valid_path_within_project() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("src/main.rs");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "fn main() {}").unwrap();
    let resolved = resolve_path(&file.to_string_lossy(), dir.path()).unwrap();
    assert!(resolved.starts_with(dir.path()));
}

#[test]
fn test_traversal_blocked() {
    let dir = TempDir::new().unwrap();
    let result = resolve_path("../../etc/passwd", dir.path());
    assert!(matches!(result, Err(PathError::TraversalBlocked(_))));
}

#[test]
fn test_absolute_path_outside_project_blocked() {
    let dir = TempDir::new().unwrap();
    let result = resolve_path("/etc/passwd", dir.path());
    assert!(matches!(result, Err(PathError::TraversalBlocked(_))));
}

#[test]
fn test_hidden_git_dir_blocked() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
    std::fs::write(dir.path().join(".git/objects/abc"), "data").unwrap();
    let result = resolve_path(".git/objects/abc", dir.path());
    assert!(matches!(result, Err(PathError::HiddenDirBlocked(_))));
}

#[test]
fn test_git_head_allowed() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    std::fs::write(dir.path().join(".git/HEAD"), "ref: refs/heads/main").unwrap();
    let result = resolve_path(".git/HEAD", dir.path());
    assert!(result.is_ok());
}

#[test]
fn test_symlink_within_project_allowed() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("real.rs");
    let link = dir.path().join("link.rs");
    std::fs::write(&target, "fn main() {}").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let resolved = resolve_path(&link.to_string_lossy(), dir.path()).unwrap();
    assert!(resolved.starts_with(dir.path()));
}

#[test]
fn test_symlink_escape_blocked() {
    let dir = TempDir::new().unwrap();
    let link = dir.path().join("escape.rs");
    std::os::unix::fs::symlink("/etc/hosts", &link).unwrap();
    let result = resolve_path(&link.to_string_lossy(), dir.path());
    assert!(matches!(result, Err(PathError::SymlinkEscape(_))));
}
