use codeaware_mcp::tools::test_selector::select_tests;
use std::fs;
use tempfile::TempDir;

fn setup_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    // Create Cargo.toml
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    // Create src file
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/auth.rs"),
        "pub fn verify_token() {}\npub fn login() {}\n",
    )
    .unwrap();
    // Create test files
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/test_auth.rs"),
        "use crate::auth;\n#[test]\nfn test_verify_token() { auth::verify_token(); }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/test_server.rs"),
        "use crate::server;\n#[test]\nfn test_handle() {}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/test_utils.rs"),
        "#[test]\nfn test_helper() {}\n",
    )
    .unwrap();
    dir
}

#[test]
fn test_finds_test_file_by_convention() {
    let dir = setup_project();
    let sel = select_tests("src/auth.rs", &[], dir.path());
    assert!(sel
        .selected_tests
        .iter()
        .any(|t| t.test_file.contains("test_auth")));
}

#[test]
fn test_matches_symbol_in_test() {
    let dir = setup_project();
    let sel = select_tests("src/auth.rs", &["verify_token".to_string()], dir.path());
    let auth_test = sel
        .selected_tests
        .iter()
        .find(|t| t.test_file.contains("test_auth"))
        .unwrap();
    assert!(auth_test.reason.contains("verify_token"));
}

#[test]
fn test_builds_cargo_test_command() {
    let dir = setup_project();
    let sel = select_tests("src/auth.rs", &["verify_token".to_string()], dir.path());
    assert!(sel.command.starts_with("cargo test"));
    assert!(sel.command.contains("test_auth"));
}

#[test]
fn test_fallback_to_full_suite() {
    let dir = setup_project();
    // Edit a file with no matching tests
    fs::write(
        dir.path().join("src/orphan.rs"),
        "fn nobody_tests_this() {}\n",
    )
    .unwrap();
    let sel = select_tests(
        "src/orphan.rs",
        &["nobody_tests_this".to_string()],
        dir.path(),
    );
    assert_eq!(sel.command, "cargo test");
    assert!(sel.coverage_estimate.contains("full suite"));
}

#[test]
fn test_coverage_estimate() {
    let dir = setup_project();
    let sel = select_tests("src/auth.rs", &["verify_token".to_string()], dir.path());
    assert!(sel.coverage_estimate.contains("of 3 test files"));
}
