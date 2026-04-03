use codeaware_mcp::tools::prefetch::prefetch_for_file;
use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;

fn setup_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("src/auth.rs"),
        "pub fn verify() {}\npub fn login() {}\n",
    )
    .unwrap();
    fs::write(dir.path().join("src/mod.rs"), "pub mod auth;\n").unwrap();
    fs::write(
        dir.path().join("tests/test_auth.rs"),
        "#[test]\nfn test_verify() {}\n",
    )
    .unwrap();
    dir
}

#[test]
fn test_prefetch_finds_test_file() {
    let dir = setup_project();
    let session = HashSet::new();
    let results = prefetch_for_file("src/auth.rs", &session, dir.path());
    assert!(results.iter().any(|r| r.path.contains("test_auth")));
}

#[test]
fn test_prefetch_skips_already_read() {
    let dir = setup_project();
    let mut session = HashSet::new();
    session.insert("tests/test_auth.rs".to_string());
    let results = prefetch_for_file("src/auth.rs", &session, dir.path());
    assert!(!results.iter().any(|r| r.path.contains("test_auth")));
}

#[test]
fn test_prefetch_extracts_symbols() {
    let dir = setup_project();
    let session = HashSet::new();
    let results = prefetch_for_file("src/auth.rs", &session, dir.path());
    let test_file = results.iter().find(|r| r.path.contains("test_auth"));
    assert!(test_file.is_some());
    assert!(test_file
        .unwrap()
        .symbols
        .contains(&"test_verify".to_string()));
}

#[test]
fn test_prefetch_max_3() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(dir.path().join("src/foo.rs"), "fn foo() {}\n").unwrap();
    for i in 0..5 {
        fs::write(
            dir.path().join(format!("tests/test_foo{}.rs", i)),
            &format!("fn test_foo{}() {{}}\n", i),
        )
        .unwrap();
    }
    let session = HashSet::new();
    let results = prefetch_for_file("src/foo.rs", &session, dir.path());
    assert!(results.len() <= 3);
}

#[test]
fn test_prefetch_finds_sibling_mod() {
    let dir = setup_project();
    let session = HashSet::new();
    let results = prefetch_for_file("src/auth.rs", &session, dir.path());
    assert!(results.iter().any(|r| r.path.contains("mod.rs")));
}
