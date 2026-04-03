use codeaware_mcp::envelope::{Envelope, ErrorCode, TrustLevel};

#[test]
fn test_success_envelope_serializes() {
    let env = Envelope::success(
        serde_json::json!({"path": "src/main.rs"}),
        TrustLevel::Exact,
    );
    let json = serde_json::to_value(&env).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["error_code"].is_null());
    assert_eq!(json["retryable"], false);
    assert!(json["trace_id"].as_str().unwrap().starts_with("t-"));
    assert_eq!(json["data"]["path"], "src/main.rs");
}

#[test]
fn test_error_envelope_serializes() {
    let env = Envelope::<()>::error(
        ErrorCode::EAmbiguousMatch,
        false,
        Some("Verwende strategy=lines".into()),
    );
    let json = serde_json::to_value(&env).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["error_code"], "E_AMBIGUOUS_MATCH");
    assert_eq!(json["fallback_suggestion"], "Verwende strategy=lines");
}

#[test]
fn test_all_error_codes_have_string_repr() {
    let codes = vec![
        ErrorCode::EAmbiguousMatch,
        ErrorCode::EStaleRead,
        ErrorCode::ESymlinkEscape,
        ErrorCode::ESecretBlocked,
        ErrorCode::ELspUnavailable,
        ErrorCode::ELspTimeout,
        ErrorCode::EParseFailed,
        ErrorCode::EBinaryFile,
        ErrorCode::EFileTooLarge,
        ErrorCode::EPermissionDenied,
        ErrorCode::ESyntaxInvalid,
        ErrorCode::EHashMismatch,
        ErrorCode::ECommandDenied,
        ErrorCode::ESqliteLocked,
        ErrorCode::EMcpVersionMismatch,
    ];
    for code in codes {
        let s = serde_json::to_value(&code).unwrap();
        assert!(s.as_str().unwrap().starts_with("E_"));
    }
}

#[test]
fn test_trust_levels_serialize() {
    assert_eq!(serde_json::to_value(TrustLevel::Exact).unwrap(), "exact");
    assert_eq!(serde_json::to_value(TrustLevel::Structural).unwrap(), "structural");
    assert_eq!(serde_json::to_value(TrustLevel::Heuristic).unwrap(), "heuristic");
    assert_eq!(serde_json::to_value(TrustLevel::Degraded).unwrap(), "degraded");
    assert_eq!(serde_json::to_value(TrustLevel::Raw).unwrap(), "raw");
}
