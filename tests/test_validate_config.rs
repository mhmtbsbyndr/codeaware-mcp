use codeaware_mcp::config::project_detect::detect_workspace;
use codeaware_mcp::tools::validate_config::{validate_config, ValidateConfigInput};
use std::fs;
use tempfile::TempDir;

// ── validate_config tests ──────────────────────────────────────────────────

#[test]
fn test_validate_missing_config() {
    let dir = TempDir::new().unwrap();
    let input = ValidateConfigInput { scope: "all".into() };
    let result = validate_config(&input, dir.path()).unwrap();
    // Should have findings about missing config
    assert!(!result.findings.is_empty());
}

#[test]
fn test_validate_good_config() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join(".codeaware.toml"),
        "[project]\nname = \"test\"\nlanguages = [\"rust\"]\n\n[compression]\nscan_secrets = true\nskeleton_threshold_loc = 50\n",
    )
    .unwrap();
    fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{"permissions":{"allow":["codeaware__smart_read"],"deny":["Bash(rm -rf *)"]}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n## Build & Test\ncargo test\n",
    )
    .unwrap();

    let input = ValidateConfigInput { scope: "all".into() };
    let result = validate_config(&input, dir.path()).unwrap();
    // Should have decent score
    assert!(result.score >= 50);
}

#[test]
fn test_validate_detects_broad_bash_permission() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{"permissions":{"allow":["Bash(*)"]}}"#,
    )
    .unwrap();

    let input = ValidateConfigInput {
        scope: "security".into(),
    };
    let result = validate_config(&input, dir.path()).unwrap();
    let codes: Vec<&str> = result.findings.iter().map(|f| f.code.as_str()).collect();
    assert!(codes.iter().any(|c| c.starts_with("SEC")));
}

#[test]
fn test_validate_detects_disabled_secrets() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".codeaware.toml"),
        "[compression]\nscan_secrets = false\n",
    )
    .unwrap();

    let input = ValidateConfigInput {
        scope: "security".into(),
    };
    let result = validate_config(&input, dir.path()).unwrap();
    assert!(result
        .findings
        .iter()
        .any(|f| f.message.contains("scan_secrets")));
}

// ── V22 structured findings tests ─────────────────────────────────────────

#[test]
fn test_validate_config_returns_score_and_grade() {
    use codeaware_mcp::tools::validate_config::run_validate_config;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let result = run_validate_config(dir.path(), "all");

    assert!(result["score"].as_u64().is_some());
    assert!(result["grade"].is_string());
    let grade = result["grade"].as_str().unwrap();
    assert!(
        ["A", "B", "C", "D", "F"].contains(&grade),
        "grade '{}' not in expected set",
        grade
    );
    assert!(result["findings"].is_array());
    assert!(result["categories"]["security"]["score"].is_number());
    assert!(result["categories"]["quality"]["score"].is_number());
    assert!(result["categories"]["efficiency"]["score"].is_number());
}

#[test]
fn test_finding_structure_has_required_fields() {
    use codeaware_mcp::tools::validate_config::run_validate_config;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    // Create a .codeaware.toml with scan_secrets = false to trigger SEC-003 finding
    fs::write(
        dir.path().join(".codeaware.toml"),
        "[compression]\nscan_secrets = false\n",
    )
    .unwrap();

    let result = run_validate_config(dir.path(), "all");
    let findings = result["findings"].as_array().unwrap();

    if !findings.is_empty() {
        let f = &findings[0];
        assert!(f["code"].is_string());
        assert!(f["severity"].is_string());
        assert!(f["file"].is_string());
        assert!(f["message"].is_string());
        assert!(f["evidence"].is_string());
        assert!(f["recommended_fix"].is_string());
        assert!(f["auto_fixable"].is_boolean());
    }
}

#[test]
fn test_sec001_detected() {
    use codeaware_mcp::tools::validate_config::run_validate_config;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"permissions": {"allow": ["Bash(*)", "Read(*)"]}}"#,
    )
    .unwrap();

    let result = run_validate_config(dir.path(), "security");
    let findings = result["findings"].as_array().unwrap();
    let codes: Vec<&str> = findings
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"SEC-001"), "SEC-001 not found in {:?}", codes);
}

// ── monorepo / workspace detection tests ──────────────────────────────────

#[test]
fn test_detect_cargo_workspace() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("crates/auth")).unwrap();
    fs::write(
        dir.path().join("crates/auth/Cargo.toml"),
        "[package]\nname = \"auth\"\n",
    )
    .unwrap();

    let info = detect_workspace(dir.path()).unwrap();
    assert!(info.is_some());
    let ws = info.unwrap();
    assert_eq!(ws.kind, "cargo");
    assert!(!ws.packages.is_empty());
}

#[test]
fn test_detect_npm_workspace() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"workspaces":["packages/*"]}"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("packages/auth")).unwrap();
    fs::write(
        dir.path().join("packages/auth/package.json"),
        r#"{"name":"auth"}"#,
    )
    .unwrap();

    let info = detect_workspace(dir.path()).unwrap();
    assert!(info.is_some());
    assert_eq!(info.unwrap().kind, "npm");
}

#[test]
fn test_no_workspace() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"single\"\n",
    )
    .unwrap();

    let info = detect_workspace(dir.path()).unwrap();
    assert!(info.is_none());
}
