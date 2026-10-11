use serde_json::json;
use std::fs;
use tempfile::TempDir;

use codeaware_mcp::tools::test_coverage_map::handle_test_coverage_map;

/// Helper: extract the envelope data from the tool response.
fn extract_data(response: &serde_json::Value) -> serde_json::Value {
    let text = response["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    envelope["data"].clone()
}

/// Create a temp project with:
///   src/lib.rs  -> fn add(...) and fn multiply(...)
///   tests/test_lib.rs -> references "add" but not "multiply"
fn setup_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create source file
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    // Create test file that only references "add"
    let tests_dir = root.join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(
        tests_dir.join("test_lib.rs"),
        r#"
#[test]
fn test_addition() {
    assert_eq!(add(2, 3), 5);
}
"#,
    )
    .unwrap();

    dir
}

#[test]
fn test_coverage_basic() {
    let dir = setup_project();
    let params = json!({ "path": dir.path().to_str().unwrap(), "language": "rust" });
    let response = handle_test_coverage_map(&params);
    let data = extract_data(&response);

    // "add" should be tested (referenced in test file), "multiply" should not
    assert_eq!(data["total_functions"], 2);
    assert_eq!(data["tested_functions"], 1);

    // Check that "multiply" is in the untested list
    let untested = data["untested"].as_array().unwrap();
    let untested_names: Vec<&str> = untested.iter().map(|u| u["name"].as_str().unwrap()).collect();
    assert!(untested_names.contains(&"multiply"));
    assert!(!untested_names.contains(&"add"));
}

#[test]
fn test_coverage_percent() {
    let dir = setup_project();
    let params = json!({ "path": dir.path().to_str().unwrap(), "language": "rust" });
    let response = handle_test_coverage_map(&params);
    let data = extract_data(&response);

    // 1 out of 2 => 50.0%
    let pct = data["coverage_percent"].as_f64().unwrap();
    assert!((pct - 50.0).abs() < 0.1, "Expected 50.0%, got {pct}");
}

#[test]
fn test_untested_list() {
    let dir = setup_project();
    let params = json!({ "path": dir.path().to_str().unwrap(), "language": "rust" });
    let response = handle_test_coverage_map(&params);
    let data = extract_data(&response);

    let untested = data["untested"].as_array().unwrap();
    assert_eq!(untested.len(), 1);
    assert_eq!(untested[0]["name"], "multiply");
    assert_eq!(untested[0]["file"], "src/lib.rs");
    // Line should be > 0
    assert!(untested[0]["line"].as_u64().unwrap() > 0);
}

#[test]
fn test_empty_project() {
    let dir = TempDir::new().unwrap();
    let params = json!({ "path": dir.path().to_str().unwrap() });
    let response = handle_test_coverage_map(&params);
    let data = extract_data(&response);

    assert_eq!(data["total_functions"], 0);
    assert_eq!(data["tested_functions"], 0);
    assert_eq!(data["coverage_percent"], 0.0);
    assert_eq!(data["files"].as_array().unwrap().len(), 0);
    assert_eq!(data["untested"].as_array().unwrap().len(), 0);
}

fn coverage_for(files: &[(&str, &str)]) -> serde_json::Value {
    let dir = TempDir::new().unwrap();
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
    }
    let response = handle_test_coverage_map(&json!({"path": dir.path().to_str().unwrap()}));
    extract_data(&response)
}

#[test]
fn test_substring_match_is_not_coverage() {
    // `renew` in a test must not count as covering production `new`.
    let data = coverage_for(&[
        ("src/lib.rs", "pub fn new() -> u32 { 1 }\n"),
        ("src/other.rs", "pub fn renew() -> u32 { 2 }\n"),
        ("tests/t.rs", "#[test]\nfn t() { renew(); }\n"),
    ]);
    assert_eq!(data["total_functions"], 2);
    assert_eq!(data["tested_functions"], 1);
    let untested = data["untested"].as_array().unwrap();
    assert_eq!(untested.len(), 1);
    assert_eq!(untested[0]["name"], "new");
    assert_eq!(untested[0]["file"], "src/lib.rs");
}

#[test]
fn test_production_code_in_file_with_inline_tests_is_counted() {
    // Production functions must not disappear just because the file has a #[cfg(test)] module.
    let data = coverage_for(&[(
        "src/lib.rs",
        "pub fn helper() -> u32 { 2 }\n\
         pub fn lonely() -> u32 { 3 }\n\
         #[cfg(test)]\n\
         mod tests {\n    use super::*;\n    #[test]\n    fn test_helper() { assert_eq!(helper(), 2); }\n}\n",
    )]);
    assert_eq!(data["total_functions"], 2, "test_helper must not count as production code");
    assert_eq!(data["tested_functions"], 1, "helper is referenced from the inline test module");
    let untested = data["untested"].as_array().unwrap();
    assert_eq!(untested.len(), 1);
    assert_eq!(untested[0]["name"], "lonely");
}

#[test]
fn test_free_test_fn_in_source_file_is_not_production() {
    let data = coverage_for(&[(
        "src/lib.rs",
        "pub fn a() -> u32 { 1 }\n#[test]\nfn check_a() { assert_eq!(a(), 1); }\n",
    )]);
    assert_eq!(data["total_functions"], 1);
    assert_eq!(data["tested_functions"], 1);
    assert!(data["untested"].as_array().unwrap().is_empty());
}
