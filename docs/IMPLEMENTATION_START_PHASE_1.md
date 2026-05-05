# Implementation Start — Phase 1

Status: active implementation planning

This document converts the roadmap and token-saving gap analysis into the first executable engineering wave.

## Phase 1 Goal

Make token savings measurable before expanding the platform.

The first implementation wave should introduce a minimal, deterministic measurement layer that records how many tokens are saved by codeaware-mcp compared with raw file/command/git output.

## Why Phase 1 Starts Here

Without measurement, every later claim is speculative.

Before implementing symbol graphs, smart resume, LSP bridges, browser summaries, or MCP routing, the project needs a stable internal accounting model:

- raw bytes;
- compressed bytes;
- estimated raw tokens;
- estimated compressed tokens;
- savings ratio;
- savings by tool;
- savings by session;
- savings by command category;
- savings by file extension/language;
- timestamped events for benchmarks.

## New Internal Module

Suggested module name:

```text
src/token_stats.rs
```

## Core Data Types

```rust
pub struct TokenEvent {
    pub id: String,
    pub trace_id: String,
    pub session_id: String,
    pub tool: String,
    pub category: TokenEventCategory,
    pub subject: String,
    pub raw_bytes: u64,
    pub compressed_bytes: u64,
    pub estimated_raw_tokens: u64,
    pub estimated_compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
    pub created_at: String,
}

pub enum TokenEventCategory {
    FileRead,
    CommandOutput,
    GitDiff,
    SearchOutput,
    MemoryResume,
    ToolSchema,
    Other,
}

pub struct TokenStatsSummary {
    pub events: u64,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
    pub by_tool: Vec<TokenStatsBucket>,
    pub by_category: Vec<TokenStatsBucket>,
}

pub struct TokenStatsBucket {
    pub name: String,
    pub events: u64,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
}
```

## Token Estimation

Start deterministic and dependency-light:

```rust
pub fn estimate_tokens(text: &str) -> u64 {
    let chars = text.chars().count() as u64;
    let words = text.split_whitespace().count() as u64;
    let by_chars = (chars + 3) / 4;
    let by_words = (words * 4 + 2) / 3;
    by_chars.max(by_words).max(1)
}
```

This is not exact tokenizer accounting, but it is stable enough for ratios and regression tests.

Future adapter:

```text
TokenizerProvider::Approximate
TokenizerProvider::Tiktoken
TokenizerProvider::ModelSpecific
```

## Storage

Add a SQLite table if the project already has SQLite persistence enabled:

```sql
CREATE TABLE IF NOT EXISTS token_events (
    id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    tool TEXT NOT NULL,
    category TEXT NOT NULL,
    subject TEXT NOT NULL,
    raw_bytes INTEGER NOT NULL,
    compressed_bytes INTEGER NOT NULL,
    estimated_raw_tokens INTEGER NOT NULL,
    estimated_compressed_tokens INTEGER NOT NULL,
    saved_tokens INTEGER NOT NULL,
    savings_ratio REAL NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_events_session ON token_events(session_id);
CREATE INDEX IF NOT EXISTS idx_token_events_tool ON token_events(tool);
CREATE INDEX IF NOT EXISTS idx_token_events_category ON token_events(category);
CREATE INDEX IF NOT EXISTS idx_token_events_created_at ON token_events(created_at);
```

If the persistence layer is not ready, start with in-memory aggregation and add SQLite in the next commit.

## New MCP Tools

### `token_stats`

Returns current session statistics.

Input:

```json
{
  "session_id": "current",
  "group_by": "tool"
}
```

Output:

```json
{
  "ok": true,
  "trust": "exact",
  "data": {
    "events": 42,
    "raw_tokens": 120000,
    "compressed_tokens": 18000,
    "saved_tokens": 102000,
    "savings_ratio": 0.85,
    "by_tool": [
      {
        "name": "smart_read",
        "events": 20,
        "raw_tokens": 70000,
        "compressed_tokens": 9000,
        "saved_tokens": 61000,
        "savings_ratio": 0.871
      }
    ]
  }
}
```

### `token_savings_report`

Returns a human-readable benchmark/report summary.

Input:

```json
{
  "scope": "session",
  "format": "markdown"
}
```

Output:

```json
{
  "ok": true,
  "trust": "exact",
  "data": {
    "markdown": "# Token Savings Report\n..."
  }
}
```

### `benchmark_compression`

Runs compression fixtures and returns deterministic benchmark metrics.

Input:

```json
{
  "fixture_dir": "benches/fixtures",
  "category": "all"
}
```

Output:

```json
{
  "ok": true,
  "trust": "exact",
  "data": {
    "fixtures": 12,
    "average_savings_ratio": 0.88,
    "results": []
  }
}
```

## Integration Points

Record token events after these tools generate compressed output:

- `smart_read`
- `smart_run`
- `git_diff`
- `project_map`
- `search_memory`
- `session_status`
- future `deep_research`

Each integration should pass both the hypothetical raw output size and the actual compressed output.

## Acceptance Criteria

- `token_stats` returns deterministic JSON.
- Repeated calls do not corrupt state.
- Token estimates are stable across platforms.
- Events include `trace_id` and `session_id`.
- Savings ratio is never NaN or infinite.
- Empty input is handled safely.
- Tests cover token estimation, ratio math, and grouping.

## Test Plan

Unit tests:

- estimate empty string;
- estimate normal prose;
- estimate code;
- calculate positive savings;
- calculate zero savings;
- calculate negative savings when compression expands output;
- group by tool;
- group by category.

Integration tests:

- run `smart_read` on a fixture;
- assert token event is recorded;
- call `token_stats`;
- assert event count and saved tokens are plausible.

## Follow-Up Issues

1. Implement `src/token_stats.rs`.
2. Wire token event recording into `smart_read`.
3. Wire token event recording into `smart_run`.
4. Add `token_stats` MCP tool.
5. Add `token_savings_report` MCP tool.
6. Add deterministic benchmark fixtures.
7. Add README benchmark section generated from fixtures.
