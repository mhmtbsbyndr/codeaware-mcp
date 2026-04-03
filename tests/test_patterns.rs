use codeaware_mcp::session::patterns::{PatternStore, PatternType};

#[test]
fn test_record_co_access() {
    let mut store = PatternStore::new();
    store.record_co_access("src/main.rs", "src/lib.rs");
    store.record_co_access("src/main.rs", "src/lib.rs");
    store.record_co_access("src/main.rs", "src/lib.rs");

    let patterns = store.get_co_access_patterns("src/main.rs");
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].0, "src/lib.rs");
    assert_eq!(patterns[0].1, 3); // evidence_count
}

#[test]
fn test_record_error_fix() {
    let mut store = PatternStore::new();
    store.record_error_fix("err_abc", "Add error handling to auth module");

    let fix = store.get_known_fix("err_abc");
    assert_eq!(fix, Some("Add error handling to auth module".to_string()));
}

#[test]
fn test_confidence_decay() {
    let mut store = PatternStore::new();
    store.record_co_access("a.rs", "b.rs");

    let confidence_before = store.get_confidence("a.rs", "b.rs").unwrap();
    store.apply_decay(0.05); // 5% decay
    let confidence_after = store.get_confidence("a.rs", "b.rs").unwrap();

    assert!(confidence_after < confidence_before);
}

#[test]
fn test_pruning_low_confidence() {
    let mut store = PatternStore::new();
    store.record_co_access("a.rs", "b.rs");

    // Decay until below threshold
    for _ in 0..50 {
        store.apply_decay(0.1);
    }

    store.prune(0.2); // Remove patterns below 0.2 confidence
    let patterns = store.get_co_access_patterns("a.rs");
    assert!(patterns.is_empty());
}

#[test]
fn test_confidence_increases_with_evidence() {
    let mut store = PatternStore::new();
    store.record_co_access("a.rs", "b.rs");
    let c1 = store.get_confidence("a.rs", "b.rs").unwrap();

    store.record_co_access("a.rs", "b.rs");
    let c2 = store.get_confidence("a.rs", "b.rs").unwrap();

    store.record_co_access("a.rs", "b.rs");
    let c3 = store.get_confidence("a.rs", "b.rs").unwrap();

    assert!(c2 > c1);
    assert!(c3 > c2);
    assert!(c3 <= 1.0);
}

#[test]
fn test_all_patterns_serializable() {
    let mut store = PatternStore::new();
    store.record_co_access("a.rs", "b.rs");
    store.record_error_fix("err1", "fix1");

    let json = store.to_json();
    assert!(json.contains("a.rs"));
    assert!(json.contains("err1"));

    let restored = PatternStore::from_json(&json).unwrap();
    assert_eq!(restored.get_known_fix("err1"), Some("fix1".to_string()));
}
