use codeaware_mcp::intelligence::lsp_client::{LspClient, LspConfig, LspError};

#[test]
fn test_lsp_config_from_json() {
    let json = r#"{
        "servers": {
            "rust": { "command": "rust-analyzer" },
            "typescript": { "command": "typescript-language-server", "args": ["--stdio"] },
            "python": { "command": "pylsp" }
        }
    }"#;
    let config = LspConfig::from_json(json).unwrap();
    assert_eq!(config.servers.len(), 3);
    assert_eq!(config.servers["rust"].command, "rust-analyzer");
    assert_eq!(config.servers["typescript"].args, vec!["--stdio"]);
}

#[test]
fn test_lsp_config_from_file_missing() {
    let config = LspConfig::from_file(std::path::Path::new("/nonexistent/.lsp.json"));
    assert!(config.is_ok());
    let c = config.unwrap();
    assert!(c.servers.is_empty());
}

#[test]
fn test_lsp_client_not_connected() {
    let client = LspClient::new();
    assert!(!client.is_connected());
    assert!(client.capabilities().is_empty());
}

#[test]
fn test_lsp_unavailable_for_language() {
    let client = LspClient::new();
    let result = client.get_definition("rust", "src/main.rs", 10, 5);
    assert!(matches!(result, Err(LspError::NotConnected)));
}

#[test]
fn test_select_intelligence_with_lsp_available() {
    use codeaware_mcp::intelligence::select_intelligence;
    use codeaware_mcp::intelligence::IntelligenceLevel;
    let level = select_intelligence("rust", true);
    assert_eq!(level, IntelligenceLevel::LSP);
}

#[test]
fn test_select_intelligence_fallback_chain() {
    use codeaware_mcp::intelligence::select_intelligence;
    use codeaware_mcp::intelligence::IntelligenceLevel;
    // LSP not available → tree-sitter for supported languages
    assert_eq!(select_intelligence("rust", false), IntelligenceLevel::TreeSitter);
    assert_eq!(select_intelligence("python", false), IntelligenceLevel::TreeSitter);
    // Unknown language → regex
    assert_eq!(select_intelligence("cobol", false), IntelligenceLevel::Regex);
}

#[test]
fn test_lsp_timeout_config() {
    let config = LspConfig::default();
    assert_eq!(config.timeout_ms, 2000); // 2s as per spec
    assert_eq!(config.max_consecutive_timeouts, 3);
}
