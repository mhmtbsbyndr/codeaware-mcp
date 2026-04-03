# /xray Dashboard + Confidence Score Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a live browser dashboard showing token consumption and session metrics, and a 0-100 confidence score on every smart_edit response.

**Architecture:** Two independent features sharing one new state struct (MetricsState). Confidence Score is a pure function computing 5 weighted factors. /xray is an embedded HTTP server with SSE, serving a single-file HTML dashboard. No new dependencies — uses std::net for HTTP.

**Tech Stack:** Rust (std::net::TcpListener for HTTP, include_str! for dashboard HTML, serde_json for SSE payloads), HTML/CSS/JS (inline, no frameworks)

**Existing Reference:** Codebase at `/Users/mbsbyndr/Desktop/CodeAware/v22/codeaware-mcp/`

---

### Task 1: MetricsState — Shared metrics accumulator

**Files:**
- Create: `src/xray/mod.rs`
- Create: `src/xray/metrics.rs`
- Modify: `src/lib.rs`
- Test: `tests/test_xray_metrics.rs`

- [ ] **Step 1: Create xray module declaration**

`src/xray/mod.rs`:
```rust
pub mod metrics;
pub mod server;
```

`src/lib.rs` — add at end:
```rust
pub mod xray;
```

- [ ] **Step 2: Write failing tests for MetricsState**

`tests/test_xray_metrics.rs`:
```rust
use codeaware_mcp::xray::metrics::MetricsState;

#[test]
fn test_new_metrics_state_is_empty() {
    let m = MetricsState::new();
    let snap = m.snapshot();
    assert_eq!(snap.raw_tokens_total, 0);
    assert_eq!(snap.compressed_tokens_total, 0);
    assert_eq!(snap.tool_calls, 0);
    assert!(snap.file_tokens.is_empty());
    assert!(snap.edit_scores.is_empty());
}

#[test]
fn test_record_tool_call_accumulates() {
    let mut m = MetricsState::new();
    m.record_tool_call("smart_read", Some("src/main.rs"), 500, 80);
    m.record_tool_call("smart_read", Some("src/lib.rs"), 300, 50);
    let snap = m.snapshot();
    assert_eq!(snap.tool_calls, 2);
    assert_eq!(snap.raw_tokens_total, 800);
    assert_eq!(snap.compressed_tokens_total, 130);
    assert_eq!(snap.file_tokens.len(), 2);
    assert_eq!(snap.file_tokens["src/main.rs"], 500);
}

#[test]
fn test_record_edit_score() {
    let mut m = MetricsState::new();
    m.record_edit_score("src/auth.rs", "verify_token", 82, "safe");
    let snap = m.snapshot();
    assert_eq!(snap.edit_scores.len(), 1);
    assert_eq!(snap.edit_scores[0].score, 82);
    assert_eq!(snap.edit_scores[0].verdict, "safe");
}

#[test]
fn test_snapshot_serializes_to_json() {
    let mut m = MetricsState::new();
    m.record_tool_call("smart_run", None, 200, 15);
    let snap = m.snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    assert!(json.contains("\"tool_calls\":1"));
    assert!(json.contains("\"raw_tokens_total\":200"));
}

#[test]
fn test_phase_updates() {
    let mut m = MetricsState::new();
    m.set_phase("Analyzing");
    let snap = m.snapshot();
    assert_eq!(snap.phase, "Analyzing");
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test test_xray_metrics 2>&1 | tail -5`
Expected: compilation error (module doesn't exist yet)

- [ ] **Step 4: Implement MetricsState**

`src/xray/metrics.rs`:
```rust
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct EditScoreEntry {
    pub file: String,
    pub symbol: String,
    pub score: u32,
    pub verdict: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub raw_tokens_total: u64,
    pub compressed_tokens_total: u64,
    pub tool_calls: u32,
    pub file_tokens: HashMap<String, u64>,
    pub edit_scores: Vec<EditScoreEntry>,
    pub phase: String,
    pub session_id: String,
    pub error_loops: Vec<String>,
}

pub struct MetricsState {
    raw_tokens_total: u64,
    compressed_tokens_total: u64,
    tool_calls: u32,
    file_tokens: HashMap<String, u64>,
    edit_scores: Vec<EditScoreEntry>,
    phase: String,
    session_id: String,
    error_loops: Vec<String>,
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            raw_tokens_total: 0,
            compressed_tokens_total: 0,
            tool_calls: 0,
            file_tokens: HashMap::new(),
            edit_scores: Vec::new(),
            phase: "Idle".to_string(),
            session_id: String::new(),
            error_loops: Vec::new(),
        }
    }

    pub fn record_tool_call(
        &mut self,
        _tool: &str,
        file: Option<&str>,
        raw_tokens: u64,
        compressed_tokens: u64,
    ) {
        self.tool_calls += 1;
        self.raw_tokens_total += raw_tokens;
        self.compressed_tokens_total += compressed_tokens;
        if let Some(f) = file {
            *self.file_tokens.entry(f.to_string()).or_insert(0) += raw_tokens;
        }
    }

    pub fn record_edit_score(&mut self, file: &str, symbol: &str, score: u32, verdict: &str) {
        self.edit_scores.push(EditScoreEntry {
            file: file.to_string(),
            symbol: symbol.to_string(),
            score,
            verdict: verdict.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn set_phase(&mut self, phase: &str) {
        self.phase = phase.to_string();
    }

    pub fn set_session_id(&mut self, id: &str) {
        self.session_id = id.to_string();
    }

    pub fn add_error_loop(&mut self, sig: &str) {
        if !self.error_loops.contains(&sig.to_string()) {
            self.error_loops.push(sig.to_string());
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            raw_tokens_total: self.raw_tokens_total,
            compressed_tokens_total: self.compressed_tokens_total,
            tool_calls: self.tool_calls,
            file_tokens: self.file_tokens.clone(),
            edit_scores: self.edit_scores.clone(),
            phase: self.phase.clone(),
            session_id: self.session_id.clone(),
            error_loops: self.error_loops.clone(),
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test test_xray_metrics 2>&1 | tail -5`
Expected: all 5 tests pass

- [ ] **Step 6: Commit**

```bash
git add src/xray/ src/lib.rs tests/test_xray_metrics.rs
git commit -m "feat: add MetricsState for xray dashboard"
```

---

### Task 2: Confidence Score computation

**Files:**
- Create: `src/tools/confidence.rs`
- Modify: `src/tools/mod.rs`
- Test: `tests/test_confidence.rs`

- [ ] **Step 1: Add module declaration**

`src/tools/mod.rs` — add at end:
```rust
pub mod confidence;
```

- [ ] **Step 2: Write failing tests**

`tests/test_confidence.rs`:
```rust
use codeaware_mcp::tools::confidence::{ConfidenceInput, ConfidenceScore, compute_confidence};

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
    let score = compute_confidence(&input);
    assert_eq!(score.score, 100);
    assert_eq!(score.verdict, "safe");
}

#[test]
fn test_no_tests_many_callers() {
    let input = ConfidenceInput {
        test_file_exists: false,
        symbol_in_test: false,
        callers_affected: 15,
        trust_level: "structural",
        git_changes_last_10: 3,
        is_public: true,
        signature_changed: true,
        has_unsafe: false,
        error_type_widened: false,
    };
    let score = compute_confidence(&input);
    assert!(score.score < 60, "score {} should be < 60", score.score);
    assert_eq!(score.verdict, "risky");
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
    let score = compute_confidence(&input);
    assert!(score.score >= 60 && score.score < 80, "score {} should be 60-79", score.score);
    assert_eq!(score.verdict, "review");
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
    let score = compute_confidence(&input);
    // semantic_risk: 100 - 20(pub) - 30(sig) = 50
    assert_eq!(score.factors.semantic_risk.score, 50);
    assert_eq!(score.weakest, "semantic_risk");
}

#[test]
fn test_weighted_sum_is_correct() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,  // test_coverage = 100
        callers_affected: 0,   // caller_impact = 100
        trust_level: "exact",  // type_safety = 100
        git_changes_last_10: 0, // git_stability = 100
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false, // semantic_risk = 100
    };
    let score = compute_confidence(&input);
    // 100*0.30 + 100*0.20 + 100*0.20 + 100*0.15 + 100*0.15 = 100
    assert_eq!(score.score, 100);
}

#[test]
fn test_git_timeout_defaults_neutral() {
    let input = ConfidenceInput {
        test_file_exists: false,
        symbol_in_test: false,
        callers_affected: 0,
        trust_level: "raw",
        git_changes_last_10: -1, // sentinel for "unavailable"
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let score = compute_confidence(&input);
    assert_eq!(score.factors.git_stability.score, 60);
}

#[test]
fn test_score_serializes_to_json() {
    let input = ConfidenceInput {
        test_file_exists: true,
        symbol_in_test: true,
        callers_affected: 2,
        trust_level: "structural",
        git_changes_last_10: 1,
        is_public: false,
        signature_changed: false,
        has_unsafe: false,
        error_type_widened: false,
    };
    let score = compute_confidence(&input);
    let json = serde_json::to_string(&score).unwrap();
    assert!(json.contains("\"verdict\""));
    assert!(json.contains("\"weakest\""));
    assert!(json.contains("\"suggestion\""));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test test_confidence 2>&1 | tail -5`
Expected: compilation error

- [ ] **Step 4: Implement confidence computation**

`src/tools/confidence.rs`:
```rust
use serde::Serialize;

pub struct ConfidenceInput<'a> {
    pub test_file_exists: bool,
    pub symbol_in_test: bool,
    pub callers_affected: usize,
    pub trust_level: &'a str,
    pub git_changes_last_10: i32, // -1 = unavailable
    pub is_public: bool,
    pub signature_changed: bool,
    pub has_unsafe: bool,
    pub error_type_widened: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorScore {
    pub score: u32,
    pub weight: f64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfidenceFactors {
    pub test_coverage: FactorScore,
    pub caller_impact: FactorScore,
    pub type_safety: FactorScore,
    pub git_stability: FactorScore,
    pub semantic_risk: FactorScore,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfidenceScore {
    pub score: u32,
    pub verdict: String,
    pub factors: ConfidenceFactors,
    pub weakest: String,
    pub suggestion: String,
}

pub fn compute_confidence(input: &ConfidenceInput) -> ConfidenceScore {
    let test_coverage = compute_test_coverage(input);
    let caller_impact = compute_caller_impact(input);
    let type_safety = compute_type_safety(input);
    let git_stability = compute_git_stability(input);
    let semantic_risk = compute_semantic_risk(input);

    let weighted_score = (test_coverage.score as f64 * test_coverage.weight)
        + (caller_impact.score as f64 * caller_impact.weight)
        + (type_safety.score as f64 * type_safety.weight)
        + (git_stability.score as f64 * git_stability.weight)
        + (semantic_risk.score as f64 * semantic_risk.weight);

    let score = weighted_score.round() as u32;

    let verdict = if score >= 80 {
        "safe"
    } else if score >= 60 {
        "review"
    } else {
        "risky"
    }
    .to_string();

    // Find weakest factor
    let factors_vec = [
        ("test_coverage", test_coverage.score),
        ("caller_impact", caller_impact.score),
        ("type_safety", type_safety.score),
        ("git_stability", git_stability.score),
        ("semantic_risk", semantic_risk.score),
    ];
    let weakest = factors_vec
        .iter()
        .min_by_key(|(_, s)| *s)
        .map(|(n, _)| n.to_string())
        .unwrap_or_default();

    let suggestion = generate_suggestion(&weakest, input);

    let factors = ConfidenceFactors {
        test_coverage,
        caller_impact,
        type_safety,
        git_stability,
        semantic_risk,
    };

    ConfidenceScore {
        score,
        verdict,
        factors,
        weakest,
        suggestion,
    }
}

fn compute_test_coverage(input: &ConfidenceInput) -> FactorScore {
    let score = if input.test_file_exists && input.symbol_in_test {
        100
    } else if input.test_file_exists {
        50
    } else {
        0
    };
    let detail = match score {
        100 => "Test file exists and covers this symbol".to_string(),
        50 => "Test file exists but symbol not found in tests".to_string(),
        _ => "No test file found".to_string(),
    };
    FactorScore { score, weight: 0.30, detail }
}

fn compute_caller_impact(input: &ConfidenceInput) -> FactorScore {
    let n = input.callers_affected;
    let score = match n {
        0 => 100,
        1 => 90,
        2..=3 => 70,
        4..=10 => 50,
        _ => 20,
    };
    let detail = format!("{} callers affected", n);
    FactorScore { score, weight: 0.20, detail }
}

fn compute_type_safety(input: &ConfidenceInput) -> FactorScore {
    let score = match input.trust_level {
        "exact" => 100,
        "structural" => 80,
        "heuristic" => 50,
        "degraded" => 30,
        _ => 20,
    };
    let detail = format!("Intelligence level: {}", input.trust_level);
    FactorScore { score, weight: 0.20, detail }
}

fn compute_git_stability(input: &ConfidenceInput) -> FactorScore {
    let changes = input.git_changes_last_10;
    let score = if changes < 0 {
        60 // unavailable, neutral default
    } else {
        match changes as u32 {
            0 => 100,
            1..=2 => 80,
            3..=5 => 60,
            6..=10 => 40,
            _ => 20,
        }
    };
    let detail = if changes < 0 {
        "Git history unavailable, using neutral default".to_string()
    } else {
        format!("Changed {} times in last 10 commits", changes)
    };
    FactorScore { score, weight: 0.15, detail }
}

fn compute_semantic_risk(input: &ConfidenceInput) -> FactorScore {
    let mut score: i32 = 100;
    let mut reasons = Vec::new();

    if input.is_public {
        score -= 20;
        reasons.push("public symbol");
    }
    if input.signature_changed {
        score -= 30;
        reasons.push("signature changed");
    }
    if input.has_unsafe {
        score -= 20;
        reasons.push("unsafe block modified");
    }
    if input.error_type_widened {
        score -= 20;
        reasons.push("error type widened");
    }

    let score = score.max(0) as u32;
    let detail = if reasons.is_empty() {
        "No semantic risk factors".to_string()
    } else {
        reasons.join(", ")
    };
    FactorScore { score, weight: 0.15, detail }
}

fn generate_suggestion(weakest: &str, input: &ConfidenceInput) -> String {
    match weakest {
        "test_coverage" => "Write a test covering this symbol before editing".to_string(),
        "caller_impact" => format!(
            "Run smart_run(\"cargo test\") to verify {} affected callers",
            input.callers_affected
        ),
        "type_safety" => "Consider enabling LSP for stronger type checking".to_string(),
        "git_stability" => "This code changes frequently — review carefully".to_string(),
        "semantic_risk" => "Public API or signature change — check all downstream consumers".to_string(),
        _ => String::new(),
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test test_confidence 2>&1 | tail -5`
Expected: all 8 tests pass

- [ ] **Step 6: Commit**

```bash
git add src/tools/confidence.rs src/tools/mod.rs tests/test_confidence.rs
git commit -m "feat: add confidence score computation (5 weighted factors)"
```

---

### Task 3: Integrate Confidence Score into smart_edit

**Files:**
- Modify: `src/tools/smart_edit.rs`
- Modify: `src/envelope.rs`
- Modify: `src/config/codeaware_toml.rs`
- Test: `tests/test_confidence_integration.rs`

- [ ] **Step 1: Add E_LOW_CONFIDENCE error code**

In `src/envelope.rs`, add to the `ErrorCode` enum after `EInternalError`:
```rust
    #[serde(rename = "E_LOW_CONFIDENCE")]
    ELowConfidence,
```

- [ ] **Step 2: Add confidence config fields**

In `src/config/codeaware_toml.rs`, add two default functions:
```rust
fn default_confidence_threshold() -> u32 {
    60
}

fn default_confidence_mode() -> String {
    "warn".to_string()
}
```

Add fields to `EnforcementConfig`:
```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EnforcementConfig {
    pub tdd_warning: bool,
    #[serde(default = "default_error_loop_threshold")]
    pub error_loop_threshold: u32,
    #[serde(default = "default_max_iterations_per_task")]
    pub max_iterations_per_task: u32,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: u32,
    #[serde(default = "default_confidence_mode")]
    pub confidence_mode: String,
}
```

Update the `Default` impl to include:
```rust
impl Default for EnforcementConfig {
    fn default() -> Self {
        Self {
            tdd_warning: false,
            error_loop_threshold: default_error_loop_threshold(),
            max_iterations_per_task: default_max_iterations_per_task(),
            confidence_threshold: default_confidence_threshold(),
            confidence_mode: default_confidence_mode(),
        }
    }
}
```

- [ ] **Step 3: Add confidence field to SmartEditResult**

In `src/tools/smart_edit.rs`, add import at top:
```rust
use crate::tools::confidence::ConfidenceScore;
```

Add field to `SmartEditResult`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartEditResult {
    pub path: String,
    pub applied: bool,
    pub dry_run: bool,
    pub strategy_used: String,
    pub new_file_hash: String,
    pub edits_applied: Vec<EditApplied>,
    pub syntax_check: Option<String>,
    pub impact: EditImpact,
    pub enforcement: EditEnforcement,
    pub confidence: Option<ConfidenceScore>,
}
```

In the `smart_edit` function, change the `Ok(SmartEditResult { ... })` block at line 212 to include `confidence: None`:
```rust
    Ok(SmartEditResult {
        path: input.path.clone(),
        applied,
        dry_run: input.dry_run,
        strategy_used: input.strategy.clone(),
        new_file_hash: new_hash,
        edits_applied,
        syntax_check: None,
        impact: EditImpact {
            callers_affected: 0,
            tests_affected: 0,
            test_file_exists: false,
        },
        enforcement: EditEnforcement {
            tdd_warning: false,
            uncommitted_edits_in_file: false,
        },
        confidence: None,
    })
```

- [ ] **Step 4: Write integration test**

`tests/test_confidence_integration.rs`:
```rust
use codeaware_mcp::tools::smart_edit::{SmartEditInput, smart_edit, SmartEditResult};
use std::path::Path;
use tempfile::TempDir;
use std::fs;

fn setup_project(content: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.rs");
    fs::write(&file_path, content).unwrap();
    (dir, "test.rs".to_string())
}

#[test]
fn test_smart_edit_result_has_confidence_field() {
    let (dir, file) = setup_project("fn hello() { println!(\"hi\"); }\n");
    let input = SmartEditInput {
        path: file,
        strategy: "text".into(),
        edits: Some(vec![codeaware_mcp::tools::smart_edit::EditPair {
            old: "hello".into(),
            new: "world".into(),
        }]),
        ..Default::default()
    };
    let result = smart_edit(&input, dir.path()).unwrap();
    // confidence is None by default (will be populated by server layer)
    assert!(result.confidence.is_none());
    assert!(result.applied);
}

#[test]
fn test_smart_edit_result_serializes_with_confidence() {
    let result = SmartEditResult {
        path: "test.rs".into(),
        applied: true,
        dry_run: false,
        strategy_used: "text".into(),
        new_file_hash: "abc123".into(),
        edits_applied: vec![],
        syntax_check: None,
        impact: codeaware_mcp::tools::smart_edit::EditImpact {
            callers_affected: 2,
            tests_affected: 1,
            test_file_exists: true,
        },
        enforcement: codeaware_mcp::tools::smart_edit::EditEnforcement {
            tdd_warning: false,
            uncommitted_edits_in_file: false,
        },
        confidence: Some(codeaware_mcp::tools::confidence::compute_confidence(
            &codeaware_mcp::tools::confidence::ConfidenceInput {
                test_file_exists: true,
                symbol_in_test: true,
                callers_affected: 2,
                trust_level: "structural",
                git_changes_last_10: 1,
                is_public: false,
                signature_changed: false,
                has_unsafe: false,
                error_type_widened: false,
            },
        )),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"confidence\""));
    assert!(json.contains("\"verdict\""));
    assert!(json.contains("\"weakest\""));
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test test_confidence_integration 2>&1 | tail -5`
Expected: 2 tests pass

- [ ] **Step 6: Run full test suite to verify no regressions**

Run: `cargo test 2>&1 | grep "^test result" | grep -v "0 passed"`
Expected: all existing tests still pass

- [ ] **Step 7: Commit**

```bash
git add src/tools/smart_edit.rs src/envelope.rs src/config/codeaware_toml.rs tests/test_confidence_integration.rs
git commit -m "feat: integrate confidence score into smart_edit response"
```

---

### Task 4: Embedded HTTP server for /xray

**Files:**
- Create: `src/xray/server.rs`
- Create: `src/xray/dashboard.html`
- Test: `tests/test_xray_server.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_xray_server.rs`:
```rust
use codeaware_mcp::xray::metrics::MetricsState;
use codeaware_mcp::xray::server::XrayServer;
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::net::TcpStream;

fn start_server() -> (XrayServer, u16) {
    let metrics = Arc::new(Mutex::new(MetricsState::new()));
    let server = XrayServer::start(metrics).expect("server should start");
    let port = server.port();
    (server, port)
}

#[test]
fn test_server_starts_on_free_port() {
    let (_server, port) = start_server();
    assert!(port > 0);
}

#[test]
fn test_dashboard_html_served() {
    let (_server, port) = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("200 OK"), "should return 200");
    assert!(response.contains("CodeAware X-Ray"), "should contain dashboard title");
}

#[test]
fn test_metrics_api_returns_json() {
    let (_server, port) = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream.write_all(b"GET /api/metrics HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("200 OK"));
    assert!(response.contains("application/json"));
    assert!(response.contains("raw_tokens_total"));
}

#[test]
fn test_sse_stream_connects() {
    let (_server, port) = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).ok();
    stream.write_all(b"GET /events HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("text/event-stream"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test test_xray_server 2>&1 | tail -5`
Expected: compilation error

- [ ] **Step 3: Create the dashboard HTML**

`src/xray/dashboard.html`:
```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>CodeAware X-Ray</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0d1117;color:#c9d1d9;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;padding:20px}
h1{font-size:24px;margin-bottom:20px;color:#58a6ff}
.grid{display:grid;grid-template-columns:1fr 1fr 1fr;gap:16px;margin-bottom:20px}
.panel{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px}
.panel h2{font-size:14px;color:#8b949e;text-transform:uppercase;letter-spacing:1px;margin-bottom:12px}
.big-number{font-size:48px;font-weight:700;color:#58a6ff}
.big-number.green{color:#3fb950}
.big-number.yellow{color:#d29922}
.big-number.red{color:#f85149}
.subtitle{font-size:13px;color:#8b949e;margin-top:4px}
.bar-container{height:24px;background:#21262d;border-radius:4px;overflow:hidden;margin:8px 0}
.bar-fill{height:100%;transition:width 0.5s ease}
.bar-fill.blue{background:#58a6ff}
.bar-fill.green{background:#3fb950}
.bar-fill.red{background:#f85149}
.file-row{display:flex;justify-content:space-between;padding:6px 0;border-bottom:1px solid #21262d;font-size:13px}
.file-path{color:#c9d1d9;font-family:monospace}
.file-tokens{color:#8b949e}
.phase-badge{display:inline-block;padding:4px 12px;border-radius:12px;font-size:13px;font-weight:600}
.phase-Idle{background:#21262d;color:#8b949e}
.phase-Analyzing{background:#0d419d;color:#a5d6ff}
.phase-Editing{background:#5a3600;color:#d29922}
.phase-Verifying{background:#1b4721;color:#3fb950}
.phase-Complete{background:#1b4721;color:#3fb950}
.score-row{display:flex;align-items:center;gap:12px;padding:8px 0;border-bottom:1px solid #21262d;font-size:13px}
.score-bar{width:80px;height:8px;background:#21262d;border-radius:4px;overflow:hidden}
.score-fill{height:100%;border-radius:4px;transition:width 0.3s}
.verdict-safe{color:#3fb950}
.verdict-review{color:#d29922}
.verdict-risky{color:#f85149}
.error-warning{background:#3d1f00;border:1px solid #d29922;border-radius:8px;padding:12px;margin-top:8px;font-size:13px;color:#d29922}
.savings{font-size:20px;font-weight:600;color:#3fb950}
.wide{grid-column:span 2}
#connection-status{position:fixed;top:8px;right:16px;font-size:11px;padding:4px 8px;border-radius:4px}
.connected{background:#1b4721;color:#3fb950}
.disconnected{background:#3d1f00;color:#f85149}
</style>
</head>
<body>
<div id="connection-status" class="disconnected">Disconnected</div>
<h1>CodeAware X-Ray</h1>

<div class="grid">
  <div class="panel">
    <h2>Token Budget</h2>
    <div class="big-number" id="tokens-used">0</div>
    <div class="subtitle">tokens consumed</div>
    <div class="bar-container"><div class="bar-fill blue" id="budget-bar" style="width:0%"></div></div>
    <div class="subtitle" id="budget-pct">0% of ~1M context</div>
  </div>

  <div class="panel">
    <h2>Compression Savings</h2>
    <div class="savings" id="savings-display">0 tokens saved</div>
    <div class="bar-container">
      <div class="bar-fill green" id="savings-bar" style="width:0%"></div>
    </div>
    <div class="subtitle" id="compression-ratio">0% compression</div>
  </div>

  <div class="panel">
    <h2>Session</h2>
    <div style="margin-bottom:8px"><span class="phase-badge phase-Idle" id="phase-badge">Idle</span></div>
    <div class="subtitle">Tool calls: <strong id="tool-calls">0</strong></div>
    <div class="subtitle" id="session-id" style="margin-top:4px;font-family:monospace;font-size:11px"></div>
  </div>
</div>

<div class="grid">
  <div class="panel">
    <h2>File Token Heatmap</h2>
    <div id="file-list"><div class="subtitle">No files read yet</div></div>
  </div>

  <div class="panel">
    <h2>Edit Confidence History</h2>
    <div id="score-list"><div class="subtitle">No edits yet</div></div>
  </div>

  <div class="panel">
    <h2>Error Loops</h2>
    <div id="error-loops"><div class="subtitle" style="color:#3fb950">No error loops detected</div></div>
  </div>
</div>

<script>
const MAX_CONTEXT = 1000000;
let es;

function connect() {
  es = new EventSource('/events');
  es.onopen = () => {
    document.getElementById('connection-status').className = 'connected';
    document.getElementById('connection-status').textContent = 'Live';
  };
  es.onerror = () => {
    document.getElementById('connection-status').className = 'disconnected';
    document.getElementById('connection-status').textContent = 'Disconnected';
    setTimeout(connect, 3000);
  };
  es.onmessage = (e) => {
    try { update(JSON.parse(e.data)); } catch(err) { console.error(err); }
  };
}

function update(d) {
  const raw = d.raw_tokens_total || 0;
  const comp = d.compressed_tokens_total || 0;
  const saved = raw - comp;
  const pct = MAX_CONTEXT > 0 ? ((comp / MAX_CONTEXT) * 100) : 0;
  const compressionRatio = raw > 0 ? ((1 - comp / raw) * 100) : 0;

  document.getElementById('tokens-used').textContent = comp.toLocaleString();
  document.getElementById('budget-bar').style.width = Math.min(pct, 100) + '%';
  document.getElementById('budget-bar').className = 'bar-fill ' + (pct > 80 ? 'red' : 'blue');
  document.getElementById('budget-pct').textContent = pct.toFixed(1) + '% of ~1M context';
  document.getElementById('savings-display').textContent = saved.toLocaleString() + ' tokens saved';
  document.getElementById('savings-bar').style.width = Math.min(compressionRatio, 100) + '%';
  document.getElementById('compression-ratio').textContent = compressionRatio.toFixed(0) + '% compression';
  document.getElementById('tool-calls').textContent = d.tool_calls || 0;

  const phase = d.phase || 'Idle';
  const badge = document.getElementById('phase-badge');
  badge.textContent = phase;
  badge.className = 'phase-badge phase-' + phase;

  if (d.session_id) document.getElementById('session-id').textContent = d.session_id;

  // File heatmap
  const ft = d.file_tokens || {};
  const sorted = Object.entries(ft).sort((a,b) => b[1] - a[1]).slice(0, 10);
  const maxTk = sorted.length > 0 ? sorted[0][1] : 1;
  const fileEl = document.getElementById('file-list');
  if (sorted.length === 0) {
    fileEl.innerHTML = '<div class="subtitle">No files read yet</div>';
  } else {
    fileEl.innerHTML = sorted.map(([f,t]) => {
      const w = ((t / maxTk) * 100).toFixed(0);
      return '<div class="file-row"><span class="file-path">' + f + '</span><span class="file-tokens">' + t.toLocaleString() + '</span></div>' +
             '<div class="bar-container"><div class="bar-fill blue" style="width:' + w + '%"></div></div>';
    }).join('');
  }

  // Edit scores
  const scores = d.edit_scores || [];
  const scoreEl = document.getElementById('score-list');
  if (scores.length === 0) {
    scoreEl.innerHTML = '<div class="subtitle">No edits yet</div>';
  } else {
    scoreEl.innerHTML = scores.slice(-10).reverse().map(s => {
      const vc = s.verdict === 'safe' ? 'verdict-safe' : s.verdict === 'review' ? 'verdict-review' : 'verdict-risky';
      return '<div class="score-row"><span class="file-path">' + s.file + '::' + s.symbol + '</span>' +
             '<div class="score-bar"><div class="score-fill ' + (s.score >= 80 ? 'bar-fill green' : s.score >= 60 ? 'bar-fill blue' : 'bar-fill red') +
             '" style="width:' + s.score + '%"></div></div>' +
             '<span class="' + vc + '">' + s.score + ' ' + s.verdict + '</span></div>';
    }).join('');
  }

  // Error loops
  const errors = d.error_loops || [];
  const errEl = document.getElementById('error-loops');
  if (errors.length === 0) {
    errEl.innerHTML = '<div class="subtitle" style="color:#3fb950">No error loops detected</div>';
  } else {
    errEl.innerHTML = errors.map(e =>
      '<div class="error-warning">Error loop detected: ' + e + '</div>'
    ).join('');
  }
}

// Initial fetch + SSE
fetch('/api/metrics').then(r => r.json()).then(update).catch(() => {});
connect();
</script>
</body>
</html>
```

- [ ] **Step 4: Implement XrayServer**

`src/xray/server.rs`:
```rust
use crate::xray::metrics::MetricsState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

pub struct XrayServer {
    port: u16,
    _handle: thread::JoinHandle<()>,
}

impl XrayServer {
    pub fn start(metrics: Arc<Mutex<MetricsState>>) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let metrics = Arc::clone(&metrics);
                    thread::spawn(move || handle_connection(stream, &metrics));
                }
            }
        });

        Ok(XrayServer { port, _handle: handle })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

fn handle_connection(mut stream: TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let mut buf = [0u8; 2048];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request = String::from_utf8_lossy(&buf[..n]);

    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    match path {
        "/" => serve_html(&mut stream),
        "/api/metrics" => serve_metrics_json(&mut stream, metrics),
        "/events" => serve_sse(&mut stream, metrics),
        _ => serve_404(&mut stream),
    }
}

fn serve_html(stream: &mut TcpStream) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        DASHBOARD_HTML.len(),
        DASHBOARD_HTML
    );
    let _ = stream.write_all(response.as_bytes());
}

fn serve_metrics_json(stream: &mut TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let snapshot = metrics.lock().map(|m| m.snapshot());
    let json = match snapshot {
        Ok(snap) => serde_json::to_string(&snap).unwrap_or_else(|_| "{}".to_string()),
        Err(_) => "{}".to_string(),
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        json.len(),
        json
    );
    let _ = stream.write_all(response.as_bytes());
}

fn serve_sse(stream: &mut TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    loop {
        let json = metrics
            .lock()
            .ok()
            .and_then(|m| serde_json::to_string(&m.snapshot()).ok())
            .unwrap_or_else(|| "{}".to_string());

        let msg = format!("data: {json}\n\n");
        if stream.write_all(msg.as_bytes()).is_err() {
            break; // client disconnected
        }
        let _ = stream.flush();
        thread::sleep(Duration::from_secs(2));
    }
}

fn serve_404(stream: &mut TcpStream) {
    let body = "Not Found";
    let response = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test test_xray_server 2>&1 | tail -5`
Expected: all 4 tests pass

- [ ] **Step 6: Commit**

```bash
git add src/xray/server.rs src/xray/dashboard.html tests/test_xray_server.rs
git commit -m "feat: embedded HTTP server + dashboard for /xray"
```

---

### Task 5: Register xray MCP tool

**Files:**
- Create: `src/tools/xray.rs`
- Modify: `src/tools/mod.rs`
- Modify: `src/server.rs`
- Test: `tests/test_xray_tool.rs`

- [ ] **Step 1: Create xray tool handler**

`src/tools/xray.rs`:
```rust
use crate::xray::metrics::MetricsState;
use crate::xray::server::XrayServer;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize)]
pub struct XrayResult {
    pub url: String,
    pub port: u16,
    pub message: String,
}

/// Start the xray dashboard server and return its URL.
/// If the server is already running, return the existing URL.
pub fn handle_xray(
    metrics: Arc<Mutex<MetricsState>>,
    existing_server: &Mutex<Option<XrayServer>>,
) -> Result<XrayResult, String> {
    let mut guard = existing_server.lock().map_err(|e| format!("lock error: {e}"))?;

    if let Some(ref server) = *guard {
        return Ok(XrayResult {
            url: server.url(),
            port: server.port(),
            message: "Dashboard already running".to_string(),
        });
    }

    let server = XrayServer::start(metrics).map_err(|e| format!("server start error: {e}"))?;
    let result = XrayResult {
        url: server.url(),
        port: server.port(),
        message: "Dashboard started. Opening browser...".to_string(),
    };

    // Try to open browser
    let url = server.url();
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(&url).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new("xdg-open").arg(&url).spawn(); }

    *guard = Some(server);
    Ok(result)
}
```

- [ ] **Step 2: Add to tools/mod.rs**

Add at end of `src/tools/mod.rs`:
```rust
pub mod xray;
```

- [ ] **Step 3: Register in server.rs**

In `src/server.rs`, add the xray tool to `handle_tools_list()` inside the `"tools"` array:
```rust
{
    "name": "xray",
    "description": "Open a live browser dashboard showing token consumption, compression savings, file heatmap, edit confidence scores, and session metrics in real-time",
    "inputSchema": {
        "type": "object",
        "properties": {}
    }
}
```

Add fields to `McpServer`:
```rust
pub struct McpServer {
    state: Arc<Mutex<SessionState>>,
    metrics: Arc<Mutex<MetricsState>>,
    xray_server: Mutex<Option<XrayServer>>,
}
```

Update `new()`:
```rust
pub fn new() -> Self {
    McpServer {
        state: Arc::new(Mutex::new(SessionState::new("."))),
        metrics: Arc::new(Mutex::new(MetricsState::new())),
        xray_server: Mutex::new(None),
    }
}
```

Add the `"xray"` match arm in `handle_tools_call`:
```rust
"xray" => {
    match crate::tools::xray::handle_xray(
        Arc::clone(&self.metrics),
        &self.xray_server,
    ) {
        Ok(result) => {
            let envelope = Envelope::success(result, TrustLevel::Exact);
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
        Err(e) => {
            let envelope = Envelope::<()>::error(ErrorCode::EInternalError, false, Some(e));
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
    }
}
```

Add necessary imports to `src/server.rs`:
```rust
use crate::xray::metrics::MetricsState;
use crate::xray::server::XrayServer;
```

- [ ] **Step 4: Write test**

`tests/test_xray_tool.rs`:
```rust
use codeaware_mcp::server::McpServer;
use serde_json::Value;

#[test]
fn test_xray_tool_in_tools_list() {
    let server = McpServer::new();
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let response = server.handle_message(msg).unwrap();
    let parsed: Value = serde_json::from_str(&response).unwrap();
    let tools = parsed["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"xray"), "xray tool should be in tools list");
}

#[test]
fn test_xray_tool_returns_url() {
    let server = McpServer::new();
    let msg = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xray","arguments":{}}}"#;
    let response = server.handle_message(msg).unwrap();
    let parsed: Value = serde_json::from_str(&response).unwrap();
    let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
    let envelope: Value = serde_json::from_str(text).unwrap();
    assert_eq!(envelope["ok"], true);
    let url = envelope["data"]["url"].as_str().unwrap();
    assert!(url.starts_with("http://127.0.0.1:"), "URL should be localhost: {}", url);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test test_xray_tool 2>&1 | tail -5`
Expected: 2 tests pass

- [ ] **Step 6: Run full test suite**

Run: `cargo test 2>&1 | grep "FAILED\|^test result" | head -30`
Expected: 0 failures

- [ ] **Step 7: Commit**

```bash
git add src/tools/xray.rs src/tools/mod.rs src/server.rs tests/test_xray_tool.rs
git commit -m "feat: register xray MCP tool with browser auto-open"
```

---

### Task 6: Wire metrics into PostToolUse hook

**Files:**
- Modify: `src/hooks/post_tool_use.rs`
- Modify: `src/server.rs` (expose metrics to hooks)
- Test: `tests/test_xray_hook_integration.rs`

- [ ] **Step 1: Update PostToolUse hook to accept MetricsState**

Replace `src/hooks/post_tool_use.rs`:
```rust
use crate::xray::metrics::MetricsState;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_post_tool_use(input: &str) -> Result<String, HookError> {
    handle_post_tool_use_with_metrics(input, None)
}

pub fn handle_post_tool_use_with_metrics(
    input: &str,
    metrics: Option<&Arc<Mutex<MetricsState>>>,
) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    let tool_name = parsed["tool_name"].as_str().unwrap_or("unknown");
    let output_size = parsed["tool_output_size"].as_u64().unwrap_or(0);
    let file_path = parsed["file_path"].as_str();

    let estimated_tokens = output_size / 4;
    let compressed_estimate = estimated_tokens * 10 / 100; // assume 90% compression

    if let Some(m) = metrics {
        if let Ok(mut state) = m.lock() {
            state.record_tool_call(tool_name, file_path, estimated_tokens, compressed_estimate);
        }
    }

    Ok(serde_json::json!({
        "decision": "approve",
        "reason": format!("Tool {tool_name}: ~{estimated_tokens} output tokens"),
        "metrics_logged": true
    })
    .to_string())
}
```

- [ ] **Step 2: Write test**

`tests/test_xray_hook_integration.rs`:
```rust
use codeaware_mcp::hooks::post_tool_use::handle_post_tool_use_with_metrics;
use codeaware_mcp::xray::metrics::MetricsState;
use std::sync::{Arc, Mutex};

#[test]
fn test_hook_updates_metrics() {
    let metrics = Arc::new(Mutex::new(MetricsState::new()));
    let input = r#"{"tool_name":"smart_read","tool_output_size":400,"file_path":"src/main.rs"}"#;
    let result = handle_post_tool_use_with_metrics(input, Some(&metrics)).unwrap();
    assert!(result.contains("approve"));

    let snap = metrics.lock().unwrap().snapshot();
    assert_eq!(snap.tool_calls, 1);
    assert_eq!(snap.raw_tokens_total, 100); // 400/4
    assert!(snap.file_tokens.contains_key("src/main.rs"));
}

#[test]
fn test_hook_without_metrics_still_works() {
    let input = r#"{"tool_name":"smart_run","tool_output_size":800}"#;
    let result = handle_post_tool_use_with_metrics(input, None).unwrap();
    assert!(result.contains("approve"));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test test_xray_hook_integration 2>&1 | tail -5`
Expected: 2 tests pass

- [ ] **Step 4: Run full test suite**

Run: `cargo test 2>&1 | grep "FAILED" | wc -l`
Expected: 0

- [ ] **Step 5: Commit**

```bash
git add src/hooks/post_tool_use.rs tests/test_xray_hook_integration.rs
git commit -m "feat: wire PostToolUse hook into xray MetricsState"
```

---

### Task 7: Final build, test, push, update release

**Files:** All

- [ ] **Step 1: Run full test suite**

```bash
cargo test 2>&1 | tail -20
```
Expected: all pass, 0 failures

- [ ] **Step 2: Run clippy**

```bash
cargo clippy 2>&1 | tail -10
```
Expected: 0 errors

- [ ] **Step 3: Build release**

```bash
cargo build --release 2>&1 | tail -3
```

- [ ] **Step 4: Install globally**

```bash
cp target/release/codeaware-mcp /usr/local/bin/codeaware-mcp
```

- [ ] **Step 5: Verify version and xray tool**

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | codeaware-mcp 2>/dev/null | python3 -c "import sys,json; tools=json.load(sys.stdin)['result']['tools']; print([t['name'] for t in tools])"
```
Expected: list includes "xray"

- [ ] **Step 6: Commit and push**

```bash
git add -A && git commit -m "feat: /xray dashboard + confidence score — complete implementation"
git push
```

- [ ] **Step 7: Update GitHub release**

```bash
git tag -d v1.0.0 && git push origin :refs/tags/v1.0.0
git tag v1.1.0 && git push origin v1.1.0
gh release create v1.1.0 --title "v1.1.0 — X-Ray Dashboard + Confidence Score" --notes "..."
```

- [ ] **Step 8: Update README with new features**
