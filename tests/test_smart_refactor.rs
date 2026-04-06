use std::fs;
use tempfile::TempDir;

use codeaware_mcp::tools::smart_refactor::handle_smart_refactor;

fn setup_temp_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    // Initialize as a git repo so the ignore crate works properly
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .ok();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&path, content).expect("Failed to write test file");
    }
    dir
}

fn extract_data(result: &serde_json::Value) -> serde_json::Value {
    let text = result["content"][0]["text"]
        .as_str()
        .expect("Missing text in result");
    let envelope: serde_json::Value =
        serde_json::from_str(text).expect("Failed to parse envelope");
    envelope
}

#[test]
fn test_rename_dry_run() {
    let dir = setup_temp_project(&[
        (
            "main.rs",
            "fn old_func() {}\nfn caller() { old_func(); }\n",
        ),
        (
            "lib.rs",
            "pub use crate::old_func;\n",
        ),
    ]);

    let params = serde_json::json!({
        "operation": "rename",
        "old_name": "old_func",
        "new_name": "new_func",
        "path": dir.path().to_str().unwrap(),
        "dry_run": true
    });

    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);

    assert!(envelope["ok"].as_bool().unwrap());
    assert!(envelope["data"]["dry_run"].as_bool().unwrap());
    assert_eq!(envelope["data"]["old_name"], "old_func");
    assert_eq!(envelope["data"]["new_name"], "new_func");
    assert_eq!(envelope["data"]["files_affected"].as_u64().unwrap(), 2);
    // main.rs has 2 occurrences (def + call), lib.rs has 1
    assert_eq!(envelope["data"]["occurrences"].as_u64().unwrap(), 3);

    // Verify files were NOT modified (dry_run)
    let main_content = fs::read_to_string(dir.path().join("main.rs")).unwrap();
    assert!(main_content.contains("old_func"));
    assert!(!main_content.contains("new_func"));
}

#[test]
fn test_rename_apply() {
    let dir = setup_temp_project(&[
        (
            "main.rs",
            "fn old_func() {}\nfn caller() { old_func(); }\n",
        ),
        (
            "lib.rs",
            "pub use crate::old_func;\n",
        ),
    ]);

    let params = serde_json::json!({
        "operation": "rename",
        "old_name": "old_func",
        "new_name": "new_func",
        "path": dir.path().to_str().unwrap(),
        "dry_run": false
    });

    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);

    assert!(envelope["ok"].as_bool().unwrap());
    assert!(!envelope["data"]["dry_run"].as_bool().unwrap());

    // Verify files WERE modified
    let main_content = fs::read_to_string(dir.path().join("main.rs")).unwrap();
    assert!(!main_content.contains("old_func"));
    assert!(main_content.contains("new_func"));
    assert!(main_content.contains("fn new_func()"));
    assert!(main_content.contains("new_func();"));

    let lib_content = fs::read_to_string(dir.path().join("lib.rs")).unwrap();
    assert!(!lib_content.contains("old_func"));
    assert!(lib_content.contains("new_func"));
}

#[test]
fn test_rename_skips_strings() {
    let dir = setup_temp_project(&[(
        "main.rs",
        r#"fn my_symbol() {}
let x = "my_symbol is a name";
// my_symbol is referenced here
let y = my_symbol();
"#,
    )]);

    let params = serde_json::json!({
        "operation": "rename",
        "old_name": "my_symbol",
        "new_name": "renamed_sym",
        "path": dir.path().to_str().unwrap(),
        "dry_run": true
    });

    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);

    assert!(envelope["ok"].as_bool().unwrap());

    // Should find occurrences on line 1 (fn def) and line 4 (call),
    // but skip line 2 (string) and line 3 (comment)
    let changes = envelope["data"]["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1); // one file
    let lines = changes[0]["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2); // line 1 and line 4 only

    let line_numbers: Vec<u64> = lines
        .iter()
        .map(|l| l["line_number"].as_u64().unwrap())
        .collect();
    assert!(line_numbers.contains(&1));
    assert!(line_numbers.contains(&4));
    assert!(!line_numbers.contains(&2)); // string
    assert!(!line_numbers.contains(&3)); // comment
}

#[test]
fn test_rename_respects_gitignore() {
    let dir = setup_temp_project(&[
        ("src/main.rs", "fn my_func() {}\n"),
        ("build/output.rs", "fn my_func() {}\n"),
        (".gitignore", "build/\n"),
    ]);

    let params = serde_json::json!({
        "operation": "rename",
        "old_name": "my_func",
        "new_name": "renamed_func",
        "path": dir.path().to_str().unwrap(),
        "dry_run": true
    });

    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);

    assert!(envelope["ok"].as_bool().unwrap());
    // Only src/main.rs should be found, not build/output.rs
    assert_eq!(envelope["data"]["files_affected"].as_u64().unwrap(), 1);

    let changes = envelope["data"]["changes"].as_array().unwrap();
    let paths: Vec<&str> = changes
        .iter()
        .map(|c| c["path"].as_str().unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.contains("src/main.rs")));
    assert!(!paths.iter().any(|p| p.contains("build/output.rs")));
}

#[test]
fn test_rename_missing_params() {
    // Missing old_name
    let params = serde_json::json!({
        "operation": "rename",
        "new_name": "something"
    });
    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);
    assert!(!envelope["ok"].as_bool().unwrap());
    assert!(envelope["fallback_suggestion"]
        .as_str()
        .unwrap()
        .contains("old_name"));

    // Missing new_name
    let params = serde_json::json!({
        "operation": "rename",
        "old_name": "something"
    });
    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);
    assert!(!envelope["ok"].as_bool().unwrap());
    assert!(envelope["fallback_suggestion"]
        .as_str()
        .unwrap()
        .contains("new_name"));
}

#[test]
fn test_rename_same_name_error() {
    let params = serde_json::json!({
        "operation": "rename",
        "old_name": "foo",
        "new_name": "foo",
        "path": "."
    });
    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);
    assert!(!envelope["ok"].as_bool().unwrap());
    assert_eq!(envelope["error_code"], "E_REFACTOR_CONFLICT");
}

#[test]
fn test_unsupported_operation() {
    let params = serde_json::json!({
        "operation": "extract_function",
        "old_name": "foo",
        "new_name": "bar"
    });
    let result = handle_smart_refactor(&params);
    let envelope = extract_data(&result);
    assert!(!envelope["ok"].as_bool().unwrap());
    assert!(envelope["fallback_suggestion"]
        .as_str()
        .unwrap()
        .contains("Unsupported operation"));
}
