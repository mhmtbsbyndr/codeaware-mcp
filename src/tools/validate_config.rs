use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
pub struct ValidateConfigInput {
    #[serde(default = "default_all")]
    pub scope: String,
}
fn default_all() -> String {
    "all".into()
}

#[derive(Debug, Serialize)]
pub struct ValidateConfigResult {
    pub ok: bool,
    pub score: u32,
    pub grade: String,
    pub findings: Vec<Finding>,
    pub categories: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub code: String,
    pub severity: String,
    pub file: String,
    pub message: String,
    pub evidence: String,
    pub recommended_fix: String,
    pub auto_fixable: bool,
}

/// Score deduction per severity (as f32 for accumulation).
fn severity_deduction(severity: &str) -> f32 {
    match severity {
        "critical" => 3.0,
        "warning" => 1.0,
        "suggestion" => 0.5,
        _ => 0.0,
    }
}

/// Public convenience wrapper that returns serde_json::Value directly.
pub fn run_validate_config(project_root: &Path, scope: &str) -> serde_json::Value {
    let input = ValidateConfigInput { scope: scope.into() };
    match validate_config(&input, project_root) {
        Ok(r) => serde_json::to_value(r).unwrap_or_else(|_| serde_json::json!({"ok": false})),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    }
}

pub fn validate_config(
    input: &ValidateConfigInput,
    project_root: &Path,
) -> Result<ValidateConfigResult, ValidateError> {
    let mut findings = Vec::new();

    let check_security = input.scope == "all" || input.scope == "security";
    let check_quality = input.scope == "all" || input.scope == "quality";
    let check_efficiency = input.scope == "all" || input.scope == "efficiency";

    if check_security {
        check_settings_json(project_root, &mut findings);
    }

    if check_quality {
        check_claude_md(project_root, &mut findings);
    }

    check_codeaware_toml(
        project_root,
        &mut findings,
        check_security,
        check_quality,
        check_efficiency,
    );

    // Calculate per-category scores
    let sec_score = category_score("security", &findings);
    let qual_score = category_score("quality", &findings);
    let eff_score = category_score("efficiency", &findings);

    let total = ((sec_score + qual_score + eff_score) as f32 / 30.0 * 100.0) as u32;
    let grade = match total {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    }
    .to_string();

    Ok(ValidateConfigResult {
        ok: findings.iter().all(|f| f.severity != "critical"),
        score: total,
        grade,
        findings,
        categories: serde_json::json!({
            "security": {"score": sec_score, "max": 10},
            "quality": {"score": qual_score, "max": 10},
            "efficiency": {"score": eff_score, "max": 10}
        }),
    })
}

/// Determine which category a finding code belongs to.
fn finding_category(code: &str) -> &'static str {
    if code.starts_with("SEC") {
        "security"
    } else if code.starts_with("QUL") {
        "quality"
    } else if code.starts_with("EFF") {
        "efficiency"
    } else {
        "quality"
    }
}

/// Compute the score (0-10) for a category given all findings.
fn category_score(category: &str, findings: &[Finding]) -> u32 {
    let deduction: f32 = findings
        .iter()
        .filter(|f| finding_category(&f.code) == category)
        .map(|f| severity_deduction(&f.severity))
        .sum();
    let score = 10.0 - deduction;
    if score < 0.0 { 0 } else { score.floor() as u32 }
}

// ── Security checks ────────────────────────────────────────────────────────

fn check_settings_json(root: &Path, findings: &mut Vec<Finding>) {
    let path = root.join(".claude/settings.json");
    if !path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            findings.push(Finding {
                code: "SEC-002".into(),
                severity: "critical".into(),
                file: ".claude/settings.json".into(),
                message: "Invalid JSON in settings.json".into(),
                evidence: e.to_string(),
                recommended_fix: "Fix JSON syntax".into(),
                auto_fixable: false,
            });
            return;
        }
    };

    // SEC-001: Bash(*) in permissions.allow
    if let Some(allow) = json
        .get("permissions")
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array())
    {
        for entry in allow {
            if let Some(s) = entry.as_str() {
                if s == "Bash(*)" || s == "Bash" {
                    findings.push(Finding {
                        code: "SEC-001".into(),
                        severity: "critical".into(),
                        file: ".claude/settings.json".into(),
                        message: "Bash(*) in permissions.allow erlaubt beliebige Shell-Befehle"
                            .into(),
                        evidence: format!("\"{}\"", s),
                        recommended_fix:
                            "Ersetze durch spezifische Patterns: Bash(cargo *), Bash(npm *)".into(),
                        auto_fixable: false,
                    });
                }
                // SEC-002: .env pattern in read permissions
                if s.contains(".env") {
                    findings.push(Finding {
                        code: "SEC-002".into(),
                        severity: "critical".into(),
                        file: ".claude/settings.json".into(),
                        message: ".env pattern in read permissions exposes secrets".into(),
                        evidence: format!("\"{}\"", s),
                        recommended_fix: "Remove .env from allowed read patterns".into(),
                        auto_fixable: false,
                    });
                }
            }
        }
    }
}

// ── Quality checks ─────────────────────────────────────────────────────────

fn check_claude_md(root: &Path, findings: &mut Vec<Finding>) {
    let path = root.join("CLAUDE.md");
    if !path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // QUL-001: CLAUDE.md > 200 lines
    let line_count = content.lines().count();
    if line_count > 200 {
        findings.push(Finding {
            code: "QUL-001".into(),
            severity: "warning".into(),
            file: "CLAUDE.md".into(),
            message: format!("CLAUDE.md hat {} Zeilen (> 200) — zu lang für effektive KI-Nutzung", line_count),
            evidence: format!("{} lines", line_count),
            recommended_fix: "Kürze CLAUDE.md auf das Wesentliche; lagere Details in separate Dateien aus".into(),
            auto_fixable: false,
        });
    }

    // QUL-002: CLAUDE.md missing build/test commands
    let has_build = content.to_lowercase().contains("build")
        || content.contains("cargo")
        || content.contains("npm")
        || content.contains("make");
    if !has_build {
        findings.push(Finding {
            code: "QUL-002".into(),
            severity: "suggestion".into(),
            file: "CLAUDE.md".into(),
            message: "CLAUDE.md fehlen Build/Test-Befehle".into(),
            evidence: String::new(),
            recommended_fix: "Füge einen '## Build & Test' Abschnitt mit Befehlen hinzu".into(),
            auto_fixable: false,
        });
    }
}

// ── .codeaware.toml checks ─────────────────────────────────────────────────

fn check_codeaware_toml(
    root: &Path,
    findings: &mut Vec<Finding>,
    check_security: bool,
    check_quality: bool,
    check_efficiency: bool,
) {
    let path = root.join(".codeaware.toml");
    if !path.exists() {
        // EFF-003: Missing .codeaware.toml entirely
        if check_efficiency {
            findings.push(Finding {
                code: "EFF-003".into(),
                severity: "warning".into(),
                file: ".codeaware.toml".into(),
                message: "Fehlende .codeaware.toml — alle Einstellungen verwenden Defaults".into(),
                evidence: String::new(),
                recommended_fix:
                    "Erstelle .codeaware.toml mit [project] und [compression] Sektionen".into(),
                auto_fixable: true,
            });
        }
        return;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // SEC-003: scan_secrets = false
    if check_security && content.contains("scan_secrets = false") {
        findings.push(Finding {
            code: "SEC-003".into(),
            severity: "warning".into(),
            file: ".codeaware.toml".into(),
            message: "scan_secrets ist deaktiviert — Secret-Scanning ausgeschaltet".into(),
            evidence: "scan_secrets = false".into(),
            recommended_fix: "Setze scan_secrets = true oder entferne die Zeile (Default: true)".into(),
            auto_fixable: true,
        });
    }

    // QUL-003: missing language config
    if check_quality && !content.contains("languages") {
        findings.push(Finding {
            code: "QUL-003".into(),
            severity: "warning".into(),
            file: ".codeaware.toml".into(),
            message: "Keine Sprachkonfiguration in .codeaware.toml".into(),
            evidence: String::new(),
            recommended_fix: "Füge languages = [\"rust\"] (oder passende Sprache) zur [project] Sektion hinzu".into(),
            auto_fixable: false,
        });
    }

    // EFF-001: missing [compression] section
    if check_efficiency && !content.contains("[compression]") {
        findings.push(Finding {
            code: "EFF-001".into(),
            severity: "suggestion".into(),
            file: ".codeaware.toml".into(),
            message: "Fehlende [compression] Sektion in .codeaware.toml".into(),
            evidence: String::new(),
            recommended_fix: "Füge [compression] Sektion hinzu um Token-Effizienz zu optimieren".into(),
            auto_fixable: true,
        });
    }

    // EFF-002: missing skeleton_threshold_loc
    if check_efficiency && !content.contains("skeleton_threshold_loc") {
        findings.push(Finding {
            code: "EFF-002".into(),
            severity: "suggestion".into(),
            file: ".codeaware.toml".into(),
            message: "skeleton_threshold_loc nicht konfiguriert".into(),
            evidence: String::new(),
            recommended_fix: "Füge skeleton_threshold_loc = 50 in [compression] hinzu".into(),
            auto_fixable: true,
        });
    }
}
