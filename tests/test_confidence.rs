use codeaware_mcp::tools::confidence::{compute_confidence, ConfidenceInput};

#[test]
fn test_all_factors_max() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,
        callers_affected: 0,
        trust_level: "exact",
        git_changes_last_10: 0,
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    assert_eq!(result.score, 100);
    assert_eq!(result.verdict, "safe");
}

#[test]
fn test_no_tests_many_callers() {
    let input = ConfidenceInput {
        test_file_exists: false,
        symbol_in_test: false,
        callers_affected: 15,
        trust_level: "heuristic",
        git_changes_last_10: 8,
        is_public: true,
        signature_changed: true,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    assert!(result.score < 60, "expected risky, got score={}", result.score);
    assert_eq!(result.verdict, "risky");
}

#[test]
fn test_medium_confidence() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: false,
        callers_affected: 3,
        trust_level: "structural",
        git_changes_last_10: 5,
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    assert!(
        result.score >= 60 && result.score <= 79,
        "expected review range 60-79, got score={}",
        result.score
    );
    assert_eq!(result.verdict, "review");
}

#[test]
fn test_public_signature_change_risky() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,
        callers_affected: 1,
        trust_level: "structural",
        git_changes_last_10: 0,
        is_public: true,
        signature_changed: true,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    assert_eq!(result.factors.semantic_risk.score, 50);
    assert_eq!(result.weakest, "semantic_risk");
}

#[test]
fn test_weighted_sum_is_correct() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,
        callers_affected: 0,
        trust_level: "exact",
        git_changes_last_10: 0,
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    // 100*0.30 + 100*0.20 + 100*0.20 + 100*0.15 + 100*0.15 = 100.0
    assert_eq!(result.score, 100);
}

#[test]
fn test_git_timeout_defaults_neutral() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,
        callers_affected: 0,
        trust_level: "exact",
        git_changes_last_10: -1,
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    assert_eq!(result.factors.git_stability.score, 60);
}

#[test]
fn test_score_serializes_to_json() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,
        callers_affected: 0,
        trust_level: "exact",
        git_changes_last_10: 0,
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let result = compute_confidence(&input);
    let json = serde_json::to_string(&result).expect("should serialize");
    assert!(json.contains("verdict"), "json missing 'verdict': {}", json);
    assert!(json.contains("weakest"), "json missing 'weakest': {}", json);
    assert!(json.contains("suggestion"), "json missing 'suggestion': {}", json);
}

#[test]
fn test_zero_score_clamps() {
    let input = ConfidenceInput {
        test_file_exists: false,
        symbol_in_test: false,
        callers_affected: 20,
        trust_level: "raw",
        git_changes_last_10: 15,
        is_public: true,
        signature_changed: true,
        has_unsafe: true,
        error_type_widened: true,
    };
    let result = compute_confidence(&input);
    // 100 - 20 - 30 - 20 - 20 = 10, but let's check it's not negative
    assert_eq!(result.factors.semantic_risk.score, 10);
    // For truly all penalties, we'd need more than 100 points of deductions
    // The current max deductions sum to 90, so minimum is 10
    // The important thing: it's >= 0 and not wrapping around
    assert!(result.factors.semantic_risk.score <= 100);
}
