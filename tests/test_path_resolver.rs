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

#[test]
fn test_symlink_dir_escape_for_nonexistent_child_blocked() {
    let base = TempDir::new().unwrap();
    let project = base.path().join("project");
    let outside = base.path().join("outside");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, project.join("link")).unwrap();
    let result = resolve_path("link/new.conf", &project);
    assert!(matches!(result, Err(PathError::SymlinkEscape(_))), "got {result:?}");
    let deep = resolve_path("link/a/b/new.conf", &project);
    assert!(matches!(deep, Err(PathError::SymlinkEscape(_))), "got {deep:?}");
}

#[test]
fn test_nonexistent_path_inside_project_allowed() {
    let dir = TempDir::new().unwrap();
    let resolved = resolve_path("src/new/file.rs", dir.path()).unwrap();
    assert!(resolved.starts_with(dir.path()));
}

#[test]
fn test_relative_path_resolved_against_root() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/a.rs"), "x").unwrap();
    let resolved = resolve_path("src/a.rs", dir.path()).unwrap();
    assert_eq!(resolved, dir.path().join("src/a.rs"));
}
