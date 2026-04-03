# v1.2.0 Features Design Spec

## Overview

Five new features for codeaware-mcp v1.2.0:

1. **Predictive Pre-fetch** — Pre-cache files Claude will likely need based on co-access history
2. **Session Timeline** — Visual timeline of tool calls in the xray dashboard
3. **Semantic Diff** — Structural change analysis on edits (not line-level)
4. **Smart Test Selector** — Identify minimal test set for a given edit
5. **Code Health Score** — Per-file health metric that evolves over sessions

---

## Feature 1: Predictive Pre-fetch

### Concept

When `smart_read("src/auth.rs")` is called, the server checks `file_access_patterns.co_accessed_with` in SQLite. If `middleware.rs` was co-accessed in 80%+ of past sessions, it pre-loads a skeleton into the session cache. When Claude requests it later, the response is instant from cache.

### Implementation

**New method in `src/session/persistence.rs`:**
```rust
pub fn get_co_accessed_files(&self, project_path: &str, file_path: &str) -> Vec<String>
```
Queries `file_access_patterns` for files with `co_accessed_with` containing `file_path`.

**New method in `src/session/persistence.rs`:**
```rust
pub fn record_co_access(&self, project_path: &str, file_a: &str, file_b: &str)
```
Updates `co_accessed_with` JSON array for both files in the pair.

**New struct `src/tools/prefetch.rs`:**
```rust
pub struct PrefetchResult {
    pub prefetched: Vec<PrefetchedFile>,
}
pub struct PrefetchedFile {
    pub path: String,
    pub mode: String,      // "skeleton"
    pub symbols: Vec<String>,
    pub reason: String,    // "co-accessed with auth.rs in 85% of sessions"
}
```

**Integration in `smart_read` response:**
Add `prefetched: Vec<PrefetchedFile>` to `SmartReadResult`. After each `smart_read`, check co-access patterns and include pre-fetched skeletons in the response.

**Co-access tracking in `handle_tools_call`:**
After each `smart_read`, record the file pair (current file + previously read files in this session) in `file_access_patterns.co_accessed_with`.

### Config
```toml
[session]
prefetch_enabled = true
prefetch_co_access_threshold = 0.7  # 70% co-access rate to trigger
prefetch_max_files = 3              # max pre-fetched files per read
```

---

## Feature 2: Session Timeline

### Concept

A new panel in the xray dashboard showing a chronological timeline of all tool calls with timestamps, durations, file paths, and token costs.

### Implementation

**Extend MetricsState** (`src/xray/metrics.rs`):
```rust
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub timestamp: String,
    pub tool: String,
    pub file: Option<String>,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub duration_ms: u64,
    pub phase: String,
}
```

Add `timeline: Vec<TimelineEvent>` to MetricsState and MetricsSnapshot.

Add `pub fn record_timeline_event(...)` method.

**Update `handle_tools_call` in `server.rs`:**
Record start time before tool execution, compute duration_ms after, push `TimelineEvent` to metrics.

**Dashboard update** (`dashboard.html`):
Add a timeline panel below the existing grid. Each event is a row with timestamp, tool icon, file path, token cost, and duration. Color-coded by tool type. Auto-scrolls to latest.

---

## Feature 3: Semantic Diff

### Concept

When `smart_edit` modifies code, instead of just returning a text diff, analyze the structural changes: return type changed, parameter added, visibility changed, symbol renamed.

### Implementation

**New struct `src/tools/semantic_diff.rs`:**
```rust
#[derive(Debug, Clone, Serialize)]
pub struct SemanticChange {
    pub change_type: String,      // "return_type_changed", "parameter_added", "visibility_changed", "symbol_renamed", "body_changed"
    pub symbol: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub breaking: bool,
    pub affected_callers: Vec<String>,
}

pub fn compute_semantic_diff(
    old_content: &str,
    new_content: &str,
    language: &str,
) -> Vec<SemanticChange>
```

**Logic:**
1. Extract symbols from old content via tree-sitter → `Vec<SymbolInfo>`
2. Extract symbols from new content via tree-sitter → `Vec<SymbolInfo>`
3. Match symbols by name
4. For each matched pair, compare: signature, kind, start_line/end_line
5. Detect: added symbols, removed symbols, signature changes, body-only changes

**Visibility detection** — extend `SymbolInfo` in `tree_sitter_provider.rs`:
Add `pub visibility: Option<String>` field. For Rust: check if node text starts with `pub`. For Python: check for `_` prefix. For TS/JS: check for `export`.

**Integration in `smart_edit` response:**
Add `semantic_changes: Vec<SemanticChange>` to `SmartEditResult`.

---

## Feature 4: Smart Test Selector

### Concept

Given a file that was edited, identify the minimal set of tests to run, instead of the full test suite.

### Implementation

**New module `src/tools/test_selector.rs`:**
```rust
pub struct TestSelection {
    pub selected_tests: Vec<SelectedTest>,
    pub command: String,           // e.g. "cargo test test_auth test_middleware"
    pub coverage_estimate: String, // "2 of 48 tests (covers edited symbols)"
}

pub struct SelectedTest {
    pub test_file: String,
    pub test_name: Option<String>,
    pub reason: String,            // "imports auth.rs", "calls verify_token"
}

pub fn select_tests(
    edited_file: &str,
    edited_symbols: &[String],
    project_root: &Path,
) -> TestSelection
```

**Logic:**
1. Find test files matching conventions: `test_*.rs`, `*_test.rs`, `tests/*.rs` (Rust), `test_*.py` (Python), `*.test.ts` (TS)
2. For each test file, grep for imports of the edited file or references to edited symbols
3. Build a `cargo test <test1> <test2>` command with only matching tests
4. If no matches found, fall back to full test suite

**Integration:**
- Add to `smart_edit` response: `suggested_tests: TestSelection`
- Add to `smart_run`: if command is `cargo test` with no args, suggest `smart_test_select` first

---

## Feature 5: Code Health Score

### Concept

Each file gets a 0-100 health score based on: test coverage, change frequency, error rate, complexity, and documentation. Persisted in SQLite, evolves over sessions.

### Implementation

**New table in `migrations/002_health_scores.sql`:**
```sql
CREATE TABLE IF NOT EXISTS code_health (
    project_path TEXT NOT NULL,
    file_path TEXT NOT NULL,
    health_score INTEGER NOT NULL DEFAULT 50,
    test_coverage_score INTEGER NOT NULL DEFAULT 50,
    stability_score INTEGER NOT NULL DEFAULT 50,
    error_score INTEGER NOT NULL DEFAULT 50,
    complexity_score INTEGER NOT NULL DEFAULT 50,
    doc_score INTEGER NOT NULL DEFAULT 50,
    last_updated TEXT NOT NULL,
    PRIMARY KEY (project_path, file_path)
);
```

**New module `src/session/health.rs`:**
```rust
pub struct CodeHealth {
    pub file_path: String,
    pub health_score: u32,
    pub factors: HealthFactors,
    pub trend: String,        // "improving", "stable", "declining"
    pub last_updated: String,
}

pub struct HealthFactors {
    pub test_coverage: u32,   // has tests? symbol covered?
    pub stability: u32,       // change frequency from git
    pub error_rate: u32,      // how often errors occur in this file
    pub complexity: u32,      // LOC, symbol count, nesting depth
    pub documentation: u32,   // doc comments present?
}
```

**Update triggers:**
- After `smart_read`: update complexity + documentation scores
- After `smart_edit`: update stability score
- After `smart_run` (test): update test_coverage + error_rate scores

**Dashboard integration:**
New "Code Health" panel in xray showing top 10 unhealthiest files with score bars and trend arrows.

**Integration in `project_map` response:**
Add `health_score: Option<u32>` to each file entry in the project map.

---

## File Structure

### New Files
| File | Feature |
|------|---------|
| `src/tools/prefetch.rs` | Predictive Pre-fetch |
| `src/tools/semantic_diff.rs` | Semantic Diff |
| `src/tools/test_selector.rs` | Smart Test Selector |
| `src/session/health.rs` | Code Health Score |
| `migrations/002_health_scores.sql` | Health score schema |

### Modified Files
| File | Changes |
|------|---------|
| `src/xray/metrics.rs` | Add TimelineEvent, timeline Vec |
| `src/xray/dashboard.html` | Add Timeline + Health panels |
| `src/server.rs` | Record timeline events, trigger health updates |
| `src/tools/smart_read.rs` | Add prefetched field to result |
| `src/tools/smart_edit.rs` | Add semantic_changes, suggested_tests |
| `src/intelligence/tree_sitter_provider.rs` | Add visibility to SymbolInfo |
| `src/session/persistence.rs` | Add co-access tracking, health CRUD |
| `src/tools/mod.rs` | Add new modules |
| `src/session/mod.rs` | Add health module |

---

## Testing

~30 new tests across:
- `tests/test_prefetch.rs` (5 tests)
- `tests/test_semantic_diff.rs` (8 tests)
- `tests/test_test_selector.rs` (5 tests)
- `tests/test_health.rs` (6 tests)
- `tests/test_timeline.rs` (4 tests)
- Integration tests in existing files

## Non-Goals
- No ML/AI in any feature — all deterministic
- No external API calls
- No new Cargo dependencies
- Health scores are heuristic, not guaranteed accurate
