# /xray Context Window Dashboard + Confidence Score — Design Spec

## Goal

Two new features for codeaware-mcp that no other MCP server has:

1. **`/xray`** — A live browser dashboard showing token consumption, compression savings, file heatmap, tool activity, session phase, and error loops in real-time.
2. **Confidence Score** — Every `smart_edit` response includes a 0-100 confidence score computed from 5 weighted factors, with configurable warn/block behavior.

## Architecture

### /xray Dashboard

codeaware-mcp gets a new MCP tool `xray` and an embedded HTTP server.

```
Claude calls xray()
    → codeaware-mcp starts HTTP server on localhost:PORT (dynamic)
    → Opens browser automatically (open / xdg-open)
    → Dashboard shows live data via SSE (Server-Sent Events)
    → Each PostToolUse hook pushes new metrics to connected clients
    → Tool response returns the dashboard URL
```

**No new dependencies.** HTTP server is ~100 lines of Rust using `std::net::TcpListener`. SSE is an HTTP response with `Content-Type: text/event-stream`. The dashboard is a single HTML file with inline CSS/JS, embedded via `include_str!()`.

**Port selection:** Bind to port 0 (OS assigns a free port), extract the actual port from the bound socket.

### Dashboard Panels

| Panel | Data | Source |
|-------|------|--------|
| **Token Budget** | Donut chart: consumed vs free, countdown to auto-compact (~95%) | PostToolUse hook accumulates `output_size / 4` estimate |
| **Compression Savings** | Bar chart: raw tokens vs compressed, estimated $ saved | smart_read/smart_run `compression_ratio` fields |
| **File Heatmap** | Top 10 files by token consumption, color = access_count | SessionState.files_read + file_access_patterns table |
| **Tool Activity** | Timeline: which tool at what time, how many tokens | session_events FTS5 table |
| **Session Phase** | Current state machine state (IDLE/ANALYZING/EDITING/VERIFYING/COMPLETE) | SessionState.phase |
| **Error Loops** | Warning when same error signature seen 3x+ | error_signatures table |
| **Edit Confidence** | Live history of smart_edit confidence scores (score bar + verdict) | Pushed from smart_edit responses |

### SSE Data Flow

```
PostToolUse hook fires
    → Calculates metrics delta (tokens consumed, compression ratio)
    → Writes to shared MetricsState (Arc<Mutex<>>)
    → SSE endpoint reads MetricsState, sends JSON event to connected browsers

smart_edit completes
    → Computes confidence score
    → Pushes score to MetricsState
    → SSE delivers to dashboard
```

### Dashboard Technology

Single HTML file (`src/xray/dashboard.html`) with:
- Inline CSS (dark theme, clean design)
- Inline JavaScript (EventSource for SSE, canvas/SVG for charts)
- No external CDN, no framework, no build step
- Responsive layout (works on any screen size)
- Embedded in binary via `include_str!()`

### HTTP Server Endpoints

| Endpoint | Method | Response |
|----------|--------|----------|
| `/` | GET | Dashboard HTML |
| `/events` | GET | SSE stream (text/event-stream) |
| `/api/metrics` | GET | Current metrics as JSON snapshot |
| `/api/history` | GET | Full session history from SQLite |

---

## Confidence Score

### Response Format

Added to every `smart_edit` response inside the `data` object:

```json
{
  "confidence": {
    "score": 82,
    "verdict": "safe",
    "factors": {
      "test_coverage":  { "score": 90, "weight": 0.30, "detail": "test_auth.rs covers this symbol" },
      "caller_impact":  { "score": 70, "weight": 0.20, "detail": "3 callers affected" },
      "type_safety":    { "score": 85, "weight": 0.20, "detail": "tree-sitter: structural" },
      "git_stability":  { "score": 75, "weight": 0.15, "detail": "changed 4 times in last 10 commits" },
      "semantic_risk":  { "score": 90, "weight": 0.15, "detail": "no public API change" }
    },
    "weakest": "caller_impact",
    "suggestion": "Run smart_run(\"cargo test\") to verify 3 affected callers"
  }
}
```

### Factor Computation

#### test_coverage (weight: 0.30)

| Condition | Score |
|-----------|-------|
| Test file exists AND symbol name found in test file | 100 |
| Test file exists (naming convention match) | 50 |
| No test file found | 0 |

Source: `EditImpact.test_file_exists` + grep for symbol name in test files.

#### caller_impact (weight: 0.20)

| Callers affected | Score |
|-----------------|-------|
| 0 | 100 |
| 1 | 90 |
| 2-3 | 70 |
| 4-10 | 50 |
| 10+ | 20 |

Source: `EditImpact.callers_affected` (already computed by smart_edit).

#### type_safety (weight: 0.20)

| Intelligence level | Score |
|-------------------|-------|
| LSP (exact) | 100 |
| tree-sitter (structural) | 80 |
| regex (heuristic) | 50 |
| raw (none) | 20 |

Source: `Envelope.trust` / `intelligence_level` from the current session.

#### git_stability (weight: 0.15)

| Changes in last 10 commits | Score |
|----------------------------|-------|
| 0 (never changed) | 100 |
| 1-2 | 80 |
| 3-5 | 60 |
| 6-10 | 40 |
| 10+ | 20 |

Source: `git log --oneline -10 -- <file>` counting commits that touch the file. Run as a subprocess with 2-second timeout. On timeout or non-git repo: score defaults to 60 (neutral).

#### semantic_risk (weight: 0.15)

Start at 100, subtract penalties:

| Condition | Penalty |
|-----------|---------|
| Public symbol (pub fn/pub struct) | -20 |
| Signature changed (parameters or return type differ) | -30 |
| Unsafe block modified | -20 |
| Error type widened | -20 |

Source: tree-sitter symbol extraction comparing old vs new content. Visibility check via `pub` keyword presence.

### Verdicts

| Score range | Verdict | Color | Behavior |
|-------------|---------|-------|----------|
| 80-100 | `safe` | Green | Edit applied, proceed |
| 60-79 | `review` | Yellow | Edit applied, manual review suggested |
| 0-59 | `risky` | Red | In `warn` mode: applied with warning. In `block` mode: rejected with `E_LOW_CONFIDENCE` |

### Configuration

```toml
[enforcement]
confidence_threshold = 60    # below this → risky verdict
confidence_mode = "warn"     # "warn" = always apply, "block" = reject risky edits
```

### New Error Code

| Code | Meaning | Retryable |
|------|---------|-----------|
| `E_LOW_CONFIDENCE` | Edit rejected because confidence < threshold and mode = block | No (must add tests or raise threshold) |

---

## File Structure

### New Files

| File | Purpose |
|------|---------|
| `src/tools/xray.rs` | MCP tool handler: starts HTTP server, returns URL |
| `src/xray/mod.rs` | Module declaration |
| `src/xray/server.rs` | Embedded HTTP server (TcpListener, SSE, static file serving) |
| `src/xray/metrics.rs` | MetricsState struct, aggregation logic, SSE serialization |
| `src/xray/dashboard.html` | Single-file HTML/CSS/JS dashboard |
| `src/tools/confidence.rs` | ConfidenceScore struct, 5-factor computation, verdict logic |

### Modified Files

| File | Change |
|------|--------|
| `src/server.rs` | Register `xray` tool in tools/list |
| `src/tools/mod.rs` | Add `pub mod xray;` and `pub mod confidence;` |
| `src/tools/smart_edit.rs` | Call `compute_confidence()`, add `confidence` field to response |
| `src/hooks/post_tool_use.rs` | Push metrics to xray MetricsState if running |
| `src/session/state.rs` | Add `raw_tokens_total` and `compressed_tokens_total` accumulators |
| `src/config/codeaware_toml.rs` | Parse `confidence_threshold` and `confidence_mode` |
| `src/envelope.rs` | Add `ELowConfidence` error code variant |
| `src/lib.rs` | Add `pub mod xray;` |
| `Cargo.toml` | No new dependencies |
| `migrations/001_initial.sql` | No schema changes (uses existing tables) |

---

## Testing

### Confidence Score Tests

| Test | Assertion |
|------|-----------|
| `test_all_factors_max` | score = 100, verdict = safe |
| `test_no_tests_no_callers` | test_coverage = 0, caller_impact = 100, score reflects |
| `test_many_callers_drops_score` | 10+ callers → caller_impact = 20 |
| `test_public_signature_change_risky` | semantic_risk = 50 (-20 pub, -30 sig change) |
| `test_block_mode_rejects_low_score` | mode = block, score < 60 → E_LOW_CONFIDENCE |
| `test_warn_mode_allows_low_score` | mode = warn, score < 60 → applied with warning |
| `test_git_timeout_defaults_neutral` | git takes >2s → git_stability = 60 |
| `test_weighted_score_calculation` | manual factor scores → verify weighted sum |

### xray Server Tests

| Test | Assertion |
|------|-----------|
| `test_server_starts_on_free_port` | bind to 0, verify port > 0 |
| `test_dashboard_html_served` | GET / returns 200 with HTML content |
| `test_metrics_api_returns_json` | GET /api/metrics returns valid JSON |
| `test_sse_stream_connects` | GET /events returns text/event-stream header |
| `test_metrics_update_propagates` | push metric → read /api/metrics → value present |

---

## Non-Goals

- No persistent storage of xray metrics across sessions (ephemeral, in-memory only)
- No authentication on the HTTP server (localhost only, session-scoped)
- No WebSocket (SSE is simpler, sufficient, and uni-directional is all we need)
- No external charting library (vanilla Canvas/SVG)
- Confidence score does not replace testing — it informs the decision to test
- No ML/AI in confidence scoring — deterministic, explainable factors only
