mod common;
use codeaware_mcp::security::secret_scanner::SecretScanner;

#[test]
fn test_detects_api_key() {
    let scanner = SecretScanner::new();
    let input = "Config loaded: API_KEY=sk-abc123def456ghi789jkl012mno345pqr";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected);
    assert!(redacted.contains("[REDACTED:"));
    assert!(!redacted.contains("sk-abc123"));
}

#[test]
fn test_detects_github_token() {
    let scanner = SecretScanner::new();
    let input = "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected);
    assert!(redacted.contains("[REDACTED:"));
}

#[test]
fn test_detects_aws_key() {
    let scanner = SecretScanner::new();
    let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected);
    assert!(redacted.contains("[REDACTED:"));
}

#[test]
fn test_detects_private_key() {
    let scanner = SecretScanner::new();
    let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQ...";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected);
    assert!(redacted.contains("[REDACTED:"));
}

#[test]
fn test_no_false_positive_on_clean_output() {
    let scanner = SecretScanner::new();
    let input = "test result: FAILED. 49 passed; 1 failed; 0 ignored";
    let (redacted, detected) = scanner.scan(input);
    assert!(!detected);
    assert_eq!(redacted, input);
}

#[test]
fn test_multiple_secrets_in_one_output() {
    let scanner = SecretScanner::new();
    let input = common::read_fixture("secret_output.txt");
    let (redacted, detected) = scanner.scan(&input);
    assert!(detected);
    assert!(!redacted.contains("sk-abc123"));
    assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(!redacted.contains("ghp_ABCDEFGHIJKLMNOP"));
    assert!(redacted.contains("Server started on port 8080"));
}

#[test]
fn test_scan_respects_max_size() {
    let scanner = SecretScanner::new();
    let huge = "x".repeat(200_000);
    let (_, detected) = scanner.scan(&huge);
    assert!(!detected);
}

#[test]
fn test_github_token_detected() {
    let scanner = SecretScanner::new();
    let input = "export TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected, "github token should be detected");
    assert!(redacted.contains("[REDACTED:"));
    assert!(!redacted.contains("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
}

#[test]
fn test_openai_key_detected() {
    let scanner = SecretScanner::new();
    let input = "OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz123456";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected, "openai key should be detected");
    assert!(redacted.contains("[REDACTED:"));
}

#[test]
fn test_stripe_key_detected() {
    let scanner = SecretScanner::new();
    // Build at runtime to avoid triggering VCS secret scanners on test fixtures
    let live_key = format!("sk{}_XXXXXXXXXXXXXXXXXXXXXXXX", "_live");
    let live_input = format!("STRIPE_SECRET={live_key}");
    let (redacted_live, detected_live) = scanner.scan(&live_input);
    assert!(detected_live, "stripe live key should be detected");
    assert!(redacted_live.contains("[REDACTED:"));

    let test_key = format!("sk{}_XXXXXXXXXXXXXXXXXXXXXXXX", "_test");
    let test_input = format!("STRIPE_SECRET={test_key}");
    let (redacted_test, detected_test) = scanner.scan(&test_input);
    assert!(detected_test, "stripe test key should be detected");
    assert!(redacted_test.contains("[REDACTED:"));
}

#[test]
fn test_jwt_detected() {
    let scanner = SecretScanner::new();
    // A realistic JWT-shaped token (header.payload.signature)
    let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let (redacted, detected) = scanner.scan(input);
    assert!(detected, "JWT should be detected");
    assert!(redacted.contains("[REDACTED:"));
}

#[test]
fn test_pattern_count_is_14() {
    let scanner = SecretScanner::new();
    assert_eq!(scanner.pattern_count(), 14, "expected exactly 14 patterns");
}
