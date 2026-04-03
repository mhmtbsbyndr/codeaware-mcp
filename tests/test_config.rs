mod common;

use codeaware_mcp::config::CodeAwareConfig;
use std::path::Path;

#[test]
fn test_parse_minimal_config() {
    let config = CodeAwareConfig::from_file(&common::fixture_path("codeaware_minimal.toml")).unwrap();
    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.project.languages, vec!["rust"]);
    // Defaults
    assert_eq!(config.compression.max_file_lines_full, 100);
    assert!(config.compression.scan_secrets);
    assert!(config.session.persistence);
}

#[test]
fn test_parse_full_config() {
    let config = CodeAwareConfig::from_file(&common::fixture_path("codeaware_full.toml")).unwrap();
    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.project.languages, vec!["rust", "typescript"]);
    assert_eq!(config.compression.max_command_output, 50);
    assert_eq!(config.enforcement.error_loop_threshold, 3);
    assert_eq!(config.languages.get("rust").unwrap().test_command, "cargo test");
}

#[test]
fn test_default_config_when_missing() {
    let config = CodeAwareConfig::from_file(Path::new("/nonexistent/.codeaware.toml")).unwrap();
    assert_eq!(config.project.name, "");
    assert!(config.project.languages.is_empty());
    assert_eq!(config.compression.max_file_lines_full, 100);
}
