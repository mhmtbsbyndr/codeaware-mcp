use codeaware_mcp::session::seen_files::SeenFiles;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_mark_seen_and_check() {
    let mut seen = SeenFiles::new();
    let hash = "abc123def456";
    seen.mark_seen("src/main.rs", hash, 1);
    assert!(seen.is_seen("src/main.rs"));
    assert!(!seen.is_seen("src/lib.rs"));
}

#[test]
fn test_stale_detection() {
    let mut seen = SeenFiles::new();
    seen.mark_seen("src/main.rs", "hash_v1", 1);
    assert!(!seen.is_stale("src/main.rs", "hash_v1"));
    assert!(seen.is_stale("src/main.rs", "hash_v2"));
}

#[test]
fn test_update_hash_on_edit() {
    let mut seen = SeenFiles::new();
    seen.mark_seen("src/main.rs", "hash_v1", 1);
    seen.update_hash("src/main.rs", "hash_v2", 2);
    assert!(!seen.is_stale("src/main.rs", "hash_v2"));
    assert_eq!(seen.last_seen_step("src/main.rs"), Some(2));
}

#[test]
fn test_invalidate_on_file_changed() {
    let mut seen = SeenFiles::new();
    seen.mark_seen("src/main.rs", "hash_v1", 1);
    seen.mark_seen("src/lib.rs", "hash_v2", 2);
    seen.invalidate("src/main.rs");
    assert!(!seen.is_seen("src/main.rs"));
    assert!(seen.is_seen("src/lib.rs"));
}

#[test]
fn test_invalidate_all_on_branch_switch() {
    let mut seen = SeenFiles::new();
    seen.mark_seen("src/main.rs", "hash_v1", 1);
    seen.mark_seen("src/lib.rs", "hash_v2", 2);
    seen.invalidate_all();
    assert!(!seen.is_seen("src/main.rs"));
    assert!(!seen.is_seen("src/lib.rs"));
}

#[test]
fn test_mark_pre_compact() {
    let mut seen = SeenFiles::new();
    seen.mark_seen("src/main.rs", "hash_v1", 1);
    seen.mark_all_pre_compact();
    assert!(seen.is_pre_compact("src/main.rs"));
    assert!(seen.is_seen("src/main.rs")); // still seen, just marked
}

#[test]
fn test_get_all_seen() {
    let mut seen = SeenFiles::new();
    seen.mark_seen("src/main.rs", "h1", 1);
    seen.mark_seen("src/lib.rs", "h2", 2);
    let all = seen.all_seen();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_compute_hash_from_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.rs");
    fs::write(&file, "fn main() {}").unwrap();
    let hash = SeenFiles::hash_file(&file).unwrap();
    assert!(!hash.is_empty());
    // Same content = same hash
    let hash2 = SeenFiles::hash_file(&file).unwrap();
    assert_eq!(hash, hash2);
    // Different content = different hash
    fs::write(&file, "fn main() { 42 }").unwrap();
    let hash3 = SeenFiles::hash_file(&file).unwrap();
    assert_ne!(hash, hash3);
}
