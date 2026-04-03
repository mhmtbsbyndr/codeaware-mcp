use codeaware_mcp::tools::smart_edit::{SmartEditInput, SmartEditResult, smart_edit, EditPair, EditImpact, EditEnforcement};
use codeaware_mcp::tools::confidence::{compute_confidence, ConfidenceInput};
use tempfile::TempDir;

fn setup_project(content: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.rs");
    std::fs::write(&file_path, content).unwrap();
    (dir, "test.rs".to_string())
}

#[test]
fn test_smart_edit_result_has_confidence_field() {
    let (dir, file) = setup_project("fn hello() { println!(\"hi\"); }\n");
    let input = SmartEditInput {
        path: file,
        strategy: "text".into(),
        edits: Some(vec![EditPair {
            old: "hello".into(),
            new: "world".into(),
        }]),
        ..Default::default()
    };
    let result = smart_edit(&input, dir.path()).unwrap();
    assert!(result.confidence.is_none());
    assert!(result.applied);
}

#[test]
fn test_smart_edit_result_serializes_with_confidence() {
    let result = SmartEditResult {
        path: "test.rs".into(),
        applied: true,
        dry_run: false,
        strategy_used: "text".into(),
        new_file_hash: "abc123".into(),
        edits_applied: vec![],
        syntax_check: None,
        impact: EditImpact {
            callers_affected: 2,
            tests_affected: 1,
            test_file_exists: true,
        },
        enforcement: EditEnforcement {
            tdd_warning: false,
            uncommitted_edits_in_file: false,
        },
        confidence: Some(compute_confidence(&ConfidenceInput {
            test_file_exists: true,
            symbol_in_test: true,
            callers_affected: 2,
            trust_level: "structural",
            git_changes_last_10: 1,
            is_public: false,
            signature_changed: false,
            has_unsafe: false,
            error_type_widened: false,
        })),
        semantic_changes: None,
        suggested_tests: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"confidence\""));
    assert!(json.contains("\"verdict\""));
    assert!(json.contains("\"weakest\""));
}
