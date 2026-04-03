mod common;
use codeaware_mcp::compressor::test_output;

#[test]
fn test_cargo_test_compression_with_failure() {
    let raw = common::read_fixture("sample_cargo_test_output.txt");
    let compressed = test_output::compress(&raw, 20);
    let lines: Vec<&str> = compressed.lines().collect();

    // Should contain summary
    assert!(compressed.contains("49 passed") || compressed.contains("1 failed"));
    // Should contain the failure details
    assert!(compressed.contains("test_error_recovery"));
    assert!(compressed.contains("assertion failed"));
    // Should NOT contain all 50 individual "ok" test lines
    assert!(lines.len() <= 20);
}

#[test]
fn test_all_passing_compression() {
    let raw = r#"running 10 tests
test test_a ... ok
test test_b ... ok
test test_c ... ok
test test_d ... ok
test test_e ... ok
test test_f ... ok
test test_g ... ok
test test_h ... ok
test test_i ... ok
test test_j ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s
"#;
    let compressed = test_output::compress(raw, 10);
    // All passing: just summary, very short
    assert!(compressed.contains("10 passed"));
    assert!(compressed.lines().count() <= 5);
}

#[test]
fn test_pytest_compression() {
    let raw = r#"============================= test session starts ==============================
platform darwin -- Python 3.11.0, pytest-7.4.0
collected 25 items

tests/test_auth.py::test_login PASSED
tests/test_auth.py::test_logout PASSED
tests/test_auth.py::test_expired_token FAILED
tests/test_db.py::test_connection PASSED

FAILED tests/test_auth.py::test_expired_token - AssertionError: Expected 401, got 200

============================== 1 failed, 24 passed ==============================
"#;
    let compressed = test_output::compress(raw, 15);
    assert!(compressed.contains("1 failed"));
    assert!(compressed.contains("test_expired_token"));
    assert!(compressed.contains("Expected 401"));
}

#[test]
fn test_short_output_not_compressed() {
    let raw = "running 2 tests\ntest a ... ok\ntest b ... ok\n\ntest result: ok. 2 passed;\n";
    let compressed = test_output::compress(raw, 50);
    // Short enough, return as-is
    assert_eq!(compressed.lines().count(), raw.lines().count());
}
