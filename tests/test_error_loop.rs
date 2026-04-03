use codeaware_mcp::tools::smart_run::compute_error_signature;

#[test]
fn test_error_signature_deterministic() {
    let output1 = "error[E0308]: mismatched types\n  --> src/main.rs:10:5\n";
    let output2 = "error[E0308]: mismatched types\n  --> src/main.rs:10:5\n";
    assert_eq!(compute_error_signature(output1), compute_error_signature(output2));
}

#[test]
fn test_different_errors_different_signatures() {
    let output1 = "error[E0308]: mismatched types\n";
    let output2 = "error[E0425]: cannot find value\n";
    assert_ne!(compute_error_signature(output1), compute_error_signature(output2));
}

#[test]
fn test_signature_ignores_timestamps() {
    let output1 = "error at 2026-04-01T12:00:00Z: connection failed\n";
    let output2 = "error at 2026-04-01T13:00:00Z: connection failed\n";
    // Signatures should be similar (timestamps stripped)
    assert_eq!(compute_error_signature(output1), compute_error_signature(output2));
}

#[test]
fn test_success_output_returns_none_signature() {
    let output = "test result: ok. 10 passed; 0 failed;\n";
    let sig = compute_error_signature(output);
    assert!(sig.is_empty());
}
