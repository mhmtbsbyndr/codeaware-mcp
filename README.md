# codeaware-mcp

A Rust MCP server that acts as a **compression and orchestration layer** between Claude Code and your filesystem. Instead of raw file content and terminal output flooding the context window, every tool call returns structured, semantically compressed results.

> **Target compression:** 70–95% token reduction depending on task type and file size. Observed benchmark range — actual results vary by codebase and workflow.

---

## Table of Contents

- [The Problem](#the-problem)
- [How It Works](#how-it-works)
- [Comparison with Other MCP Servers](#comparison-with-other-mcp-servers)
- [MCP Protocol](#mcp-protocol)
  - [Transport](#transport)
  - [Response Envelope](#response-envelope)
  - [Error Codes](#error-codes)
  - [Trust Levels](#trust-levels)
- [Tools](#tools)
- [Routing Policy](#routing-policy)
- [Code Intelligence](#code-intelligence)
- [Session State Machine](#session-state-machine)
- [Compaction Recovery](#compaction-recovery)
- [Security](#security)
- [Plugin Distribution](#plugin-distribution)
  - [Embedded Mode](#embedded-mode)
  - [Plugin Mode](#plugin-mode)
- [Skills and Agents](#skills-and-agents)
- [Hooks](#hooks)
- [Configuration](#configuration)
- [Install](#install)
- [Platform Support](#platform-support)
- [Tests](#tests)

---

## The Problem

Claude Code's built-in tools return raw content:

```
Read("src/server.rs")         → 620 lines of source in context
Bash("cargo test")            → 300+ lines of test output
Read("src/server.rs") again   → 620 lines again, nothing changed
/compact                      → all working context gone
```

Every token consumed is money spent and context window exhausted. On larger codebases, Claude regularly hits the 95%-capacity auto-compact threshold mid-task, losing all working state.

## How It Works

codeaware-mcp sits in the stdio path between Claude and the filesystem. Every request goes through three layers before returning:

**1. Intelligence layer** — tree-sitter parses the file and extracts symbols, imports, caller chains. Returns structure, not source.

**2. Compression layer** — command output is classified (test runner / compiler / linter / git / etc.) and reduced to the signal: failed tests, error lines, changed files.

**3. Session layer** — every result is indexed in SQLite FTS5. The server tracks what it already delivered. Second read of the same file → only the diff. After `/compact` → BM25 search retrieves only the relevant prior context.

```
smart_read("src/server.rs", mode="skeleton")
→ {
    symbols: ["handle_request", "route_tool", "McpServer"],
    imports: ["serde_json", "tokio"],
    loc: 620,
    stale: false,
    suggested_next: ["src/tools/mod.rs"]
  }
  (not 620 lines)

smart_run("cargo test")
→ {
    status: "fail",
    category: "test_runner",
    passed: 41,
    failed: 2,
    failures: [
      { test: "test_path_traversal", message: "assertion failed: result.is_err()" }
    ]
  }
  (not 300 lines)
```

---

## Comparison with Other MCP Servers

Three existing token-optimization MCP servers were evaluated: **Context Mode**, **Token Optimizer MCP**, and **CC Token Saver**. codeaware-mcp adopts the best ideas from each and adds capabilities none of them have.

| Feature | Context Mode | Token Optimizer | CC Token Saver | **codeaware-mcp** |
|---------|:---:|:---:|:---:|:---:|
| File read compression | — | ~80% (chunking) | — | **90–95%** (AST skeleton/focused) |
| Test output compression | — | — | — | **~92%** (failures only) |
| Build output compression | — | — | — | **~93%** (errors only) |
| Session recovery after `/compact` | ✅ | memory store | — | ✅ **full restore** |
| Code understanding depth | — | regex signatures | — | **LSP + tree-sitter** |
| Caller chains | — | — | — | ✅ |
| Impact analysis on edits | — | — | — | ✅ |
| Transactional edits (hash guard) | — | — | — | ✅ |
| Trust levels on responses | — | — | — | ✅ |
| Skills + Agents + Hooks | partial | — | — | ✅ full |
| Platforms supported | 5 | 3 | 1 | **5** |
| Local LLM for summarization | — | — | required | optional |
| Written in | — | — | — | **Rust** (single binary) |

### What codeaware-mcp does that others don't

**AST-based compression, not chunking.** Token Optimizer chunks files arbitrarily. codeaware-mcp uses tree-sitter to understand the code structure and delivers only what Claude needs — symbols, imports, call relationships — without the source lines.

**Impact analysis before edits.** Before applying a `smart_edit`, the server returns the list of functions that call the changed symbol, the test files that cover it, and a syntax validity check. Claude decides whether to proceed with full context. No other MCP server does this.

**Transactional edits with hash guard.** Every edit includes an `expected_hash` of the file's current state. If the file changed between `smart_read` and `smart_edit` (another tool ran, an external process wrote to it), the edit is rejected atomically — no silent overwrites, no data loss.

**Trust levels.** Every response includes an `intelligence_level` field: `lsp` / `tree-sitter` / `regex` / `raw`. Claude knows exactly how reliable the symbol analysis is and can calibrate its confidence accordingly.

**Compaction recovery with semantic richness.** Context Mode also does compaction recovery, but with raw text. codeaware-mcp's snapshots include symbols, caller relationships, impact-analysis results, and trust levels. The restore is semantically richer.

**Adopted from Context Mode:** FTS5 session event indexing, BM25 compaction recovery, multi-platform MCP support.
**Adopted from Token Optimizer:** Delta diffing, dollar-cost tracking, structural signature extraction (improved with tree-sitter).

---

## MCP Protocol

### Transport

codeaware-mcp speaks **JSON-RPC 2.0 over stdio**. The client writes a JSON object per line, the server responds with a JSON object per line. No HTTP, no ports, no network.

```
stdin  → {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"smart_read","arguments":{...}}}
stdout → {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"{...}"}]}}
```

Configuration in `.mcp.json`:
```json
{
  "mcpServers": {
    "codeaware": {
      "command": "codeaware-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

The server also handles hook events via the `hook` subcommand:
```bash
echo '{"event":"PostToolUse","tool":"smart_read",...}' | codeaware-mcp hook PostToolUse
```

### Response Envelope

Every tool response is wrapped in a standard envelope:

```json
{
  "ok": true,
  "error_code": null,
  "retryable": false,
  "fallback_suggestion": null,
  "trust": "tree-sitter",
  "trace_id": "t-20260401-1234-abcd",
  "data": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `ok` | boolean | `true` on success, `false` on error |
| `error_code` | string\|null | Standardized error code (see below) |
| `retryable` | boolean | Whether the agent may retry the same call |
| `fallback_suggestion` | string\|null | e.g. `"Use native Read"` or `"strategy=lines instead of text"` |
| `trust` | enum | Reliability of structural information in this response |
| `trace_id` | string | Unique ID for audit logging and debugging |
| `data` | object | Tool-specific response payload |

### Error Codes

| Code | Meaning | Retryable | Suggested fallback |
|------|---------|-----------|--------------------|
| `E_AMBIGUOUS_MATCH` | `old` text found 0× or >1× in file | No | Use `strategy=lines` or `symbol` |
| `E_STALE_READ` | File changed since last read | Yes (auto-refresh) | Run `smart_read` again |
| `E_HASH_MISMATCH` | `expected_hash` doesn't match current file (concurrent edit) | Yes (after re-read) | Re-read, then re-edit |
| `E_SYNTAX_INVALID` | Edit would produce invalid syntax (edit not applied) | No | Fix the code change |
| `E_PATH_TRAVERSAL` | Path escapes project root or hits deny list | No | — |
| `E_SYMLINK_ESCAPE` | Symlink target is outside project | No | Use native Read with warning |
| `E_SECRET_BLOCKED` | Secret detected in output, redacted | No | — |
| `E_BINARY_FILE` | File is not UTF-8 | No | Skip |
| `E_FILE_TOO_LARGE` | File exceeds `max_indexable_loc` | No | Outline only |
| `E_PERMISSION_DENIED` | OS-level or deny-list block | No | — |
| `E_LSP_UNAVAILABLE` | LSP server unreachable | No (session-wide) | tree-sitter fallback |
| `E_LSP_TIMEOUT` | LSP request exceeded 2s | Yes (once) | tree-sitter for this call |
| `E_PARSE_FAILED` | tree-sitter cannot parse file | No | Regex fallback |
| `E_COMMAND_DENIED` | Command is on deny list | No | — |
| `E_SQLITE_LOCKED` | Persistence unavailable | No (skip persistence) | — |
| `E_INVALID_SLOT` | Unknown workspace_state slot name | No | Use valid slot name |
| `E_INVALID_SLOT_VALUE` | Slot value fails schema validation | No | Check slot schema |

### Trust Levels

| Level | Meaning | Source |
|-------|---------|--------|
| `exact` | Fully correct analysis | LSP with successful type check |
| `structural` | Syntactically correct, semantics unverified | tree-sitter AST |
| `heuristic` | Pattern-based, may contain errors | Regex fallback, import heuristics |
| `degraded` | Partial information, known gaps | LSP after timeout, partial parse |
| `raw` | No structural analysis, raw data only | Unknown language, total fallback |

---

## Tools

### `project_map` — Compressed project overview

Replaces `find . -type f` + repeated `cat`.

**Input:**
```json
{
  "path": ".",
  "depth": 3,
  "include_symbols": true,
  "filter_language": "rust",
  "task_context": "Fix authentication bug"
}
```

`task_context` causes the map to rank files by relevance to the current task (files matching keywords get higher `access_frequency`).

**Output:**
```json
{
  "project_name": "my-app",
  "language": "rust",
  "total_files": 23,
  "total_loc": 4821,
  "tree": [
    {
      "path": "src/auth.rs",
      "loc": 487,
      "access_frequency": 0.9,
      "symbols": ["AuthManager", "login", "verify_token"],
      "last_modified": "2m ago"
    }
  ],
  "dependencies": { "axum": "0.7", "sqlx": "0.7" },
  "suggested_start": ["src/auth.rs"],
  "sparse_paths_recommendation": ["src/auth/", "tests/auth/"]
}
```

`sparse_paths_recommendation` can be passed to `--worktree sparsePaths` to avoid checking out the full repo.

---

### `smart_read` — Context-aware file reading

Replaces `cat file.rs`.

**Input:**
```json
{
  "path": "src/server.rs",
  "mode": "auto",
  "focus": "handle_request",
  "lines": "100-150",
  "scope": "function"
}
```

| Mode | Triggered when | Returns |
|------|---------------|---------|
| `skeleton` | File > 100 LOC (auto default) | Symbols, imports, structure — no source |
| `focused` | `focus` field set | The named function + its direct callees only |
| `full` | File ≤ 50 LOC or explicit | Full source, hash-tracked |
| `diff` | File was already read this session | Only what changed since last read |
| `auto` | Default | Selects skeleton/full based on file size |

**Output:**
```json
{
  "path": "src/server.rs",
  "mode_used": "skeleton",
  "file_hash": "a3f9b2c1...",
  "loc": 620,
  "stale": false,
  "truncated": false,
  "intelligence_level": "tree-sitter",
  "symbols": [
    { "name": "handle_request", "kind": "function", "line": 42, "start_line": 42, "end_line": 87 }
  ],
  "imports": ["serde_json", "crate::security"],
  "callers": [{ "name": "run_server", "file": "src/main.rs", "line": 15 }],
  "relevant_tests": ["tests/test_server.rs"],
  "content": null,
  "suggested_next": ["src/tools/mod.rs"]
}
```

---

### `smart_edit` — Edit with impact analysis

Replaces direct `Edit` for files where impact analysis matters.

**Input:**
```json
{
  "path": "src/server.rs",
  "mode": "text",
  "old": "fn handle_request(",
  "new": "fn handle_request_v2(",
  "expected_hash": "a3f9b2c1..."
}
```

Edit modes:

| Mode | Description |
|------|-------------|
| `text` | Exact old→new string replacement (fails if ambiguous) |
| `symbol` | Replace an entire named symbol (function/struct/class body) |
| `lines` | Replace a specific line range |

**Before applying the edit**, the response includes:

```json
{
  "applied": true,
  "new_hash": "f1c2d3e4...",
  "affected_callers": [
    { "name": "route_tool", "file": "src/server.rs", "line": 120 },
    { "name": "test_server", "file": "tests/test_server.rs", "line": 8 }
  ],
  "relevant_tests": ["tests/test_server.rs"],
  "validity": "valid",
  "suggested_next": ["smart_run(\"cargo test\")"]
}
```

If `expected_hash` doesn't match the current file state, the edit is rejected with `E_HASH_MISMATCH` — no partial writes.

---

### `smart_run` — Compressed command output

Replaces `Bash` for verbose commands.

**Input:**
```json
{
  "command": "cargo test",
  "cwd": ".",
  "env": {},
  "timeout_ms": 30000
}
```

The server classifies the command into one of 8 categories and applies category-specific compression:

| Category | Detection | Compression strategy |
|----------|-----------|---------------------|
| `test_runner` | `cargo test`, `pytest`, `jest`, `go test` | Failed tests only + assertion messages |
| `compiler` | `cargo build`, `gcc`, `tsc` | Errors + file:line refs, warnings summarized |
| `linter` | `clippy`, `eslint`, `ruff` | Grouped by rule, count per file |
| `git` | `git diff`, `git log`, `git status` | Structured summary, not raw diff |
| `package_mgr` | `cargo add`, `npm install`, `pip install` | Added/removed packages only |
| `formatter` | `rustfmt`, `prettier`, `black` | Changed file count only |
| `search` | `grep`, `rg`, `find` | Match count + top N results |
| `generic` | Everything else | Last N lines + exit code |

**Output (test_runner):**
```json
{
  "exit_code": 1,
  "category": "test_runner",
  "compressed": true,
  "status": "fail",
  "passed": 41,
  "failed": 2,
  "duration_ms": 843,
  "failures": [
    {
      "test": "test_path_traversal",
      "file": "tests/test_security.rs",
      "line": 23,
      "message": "assertion `left == right` failed\n  left: Ok(())\n right: Err(PathTraversal)"
    }
  ],
  "summary": "2 failed, 41 passed in 0.84s"
}
```

---

### `workspace_state` — Typed session slots

Five named slots for sharing state between tool calls within a session.

**Input:**
```json
{
  "action": "write",
  "slot": "active_task",
  "value": { "description": "Fix auth bug", "target_file": "src/auth.rs" }
}
```

| Action | Description |
|--------|-------------|
| `read` | Read one or all slots |
| `write` | Write to a slot (schema-validated) |
| `clear` | Clear a slot |

**Slots and their schemas:**

| Slot | Schema | Purpose |
|------|--------|---------|
| `recent_targets` | `{ "files": ["path", ...] }` | Files currently being worked on |
| `error_signatures` | `{ "errors": [{ "hash": "...", "message": "...", "fix": "..." }] }` | Recurring error patterns |
| `co_access_candidates` | `{ "pairs": [["file_a", "file_b"], ...] }` | Files that tend to be edited together |
| `verification_state` | `{ "status": "pass\|fail\|unknown", "checks": [...] }` | Current test/build pass/fail state |
| `active_task` | `{ ... }` | Arbitrary task context object |

**Incremental delivery:** First read returns `full: true` with all slot data. Subsequent reads return only slots that changed (`full: false`). Reduces repeated state reads to near zero tokens.

---

### `validate_config` — Structured config findings

Analyzes `.codeaware.toml` (or another config file) and returns structured findings, not a prose description.

**Input:**
```json
{ "path": ".codeaware.toml", "scope": "all" }
```

**Output:**
```json
{
  "ok": false,
  "score": 7.5,
  "grade": "B",
  "findings": [
    {
      "code": "SEC-001",
      "severity": "critical",
      "file": ".codeaware.toml",
      "message": "Secret scanner disabled",
      "evidence": "scan_secrets = false",
      "recommended_fix": "Remove scan_secrets = false or set to true",
      "auto_fixable": true
    }
  ],
  "categories": {
    "security": 7.0,
    "quality": 9.0,
    "efficiency": 8.5
  }
}
```

Finding codes: `SEC-001/002/003` (security), `QUL-001/002/003` (quality), `EFF-001/002/003` (efficiency).

Grading: 90+ = A, 80+ = B, 70+ = C, 60+ = D, <60 = F. Score is per-category, penalties: critical = −3, warning = −1, suggestion = −0.5.

---

### `session_status` — Session summary and compaction recovery

Returns current session state: state machine phase, files read/edited, error patterns seen, token estimates, and (after `/compact`) the most relevant prior context retrieved via FTS5 BM25 search.

---

## Routing Policy

codeaware-mcp supplements Claude's native tools — it does not replace them. The routing rules:

| Situation | Use | Reason |
|-----------|-----|--------|
| File < 50 LOC | native `Read` | Compression overhead not worth it |
| File ≥ 50 LOC | `smart_read` | Skeleton/Focused saves 70–95% |
| Need exact raw text (regex, grep) | native `Read` | Compression would lose information |
| Project structure overview | `project_map` | Compact structure vs find+cat |
| Small localized change (< 20 LOC, 1 file) | native `Edit` | No impact overhead needed |
| Edit where callers/tests matter | `smart_edit` | Impact analysis in response |
| Short command (ls, pwd, git status) | native `Bash` | No compressible output |
| Verbose test/build/lint output | `smart_run` | 80–95% compression |
| Command not in compression logic | native `Bash` | Generic truncation would be worse |
| Subtask > 5 steps or > 3 files | Subagent | Keep main context clean |

**When NOT to compress:**

- Config files (YAML, TOML, JSON) — every line can be semantically relevant
- Security reviews requiring exact string matching
- Lock files (Cargo.lock, package-lock.json) — too large, too little signal per line
- Generated code (`@generated`, `DO NOT EDIT`) — edits are meaningless
- Binary files

If `information_loss_rate` (re-reads after skeleton) exceeds 5%, the routing threshold is lowered automatically.

---

## Code Intelligence

Three tiers, descending quality:

### Tier 1: LSP (preferred for typed languages)

When a language server is running, codeaware-mcp uses it for:
- Go-to-definition
- References and call hierarchy
- Type errors after edits
- Symbol search

Configure in `.lsp.json`:
```json
{
  "servers": {
    "rust": { "command": "rust-analyzer" },
    "typescript": { "command": "typescript-language-server", "args": ["--stdio"] },
    "python": { "command": "pylsp" }
  }
}
```

LSP has a 2-second hard timeout per request. After 3 consecutive timeouts, the server falls back to tree-sitter for the rest of the session.

If a native Claude Code code-intelligence plugin is already active, codeaware-mcp delegates to it instead of running a second LSP client — no double overhead.

### Tier 2: tree-sitter (always available)

Compiled grammars for: **Rust, Python, TypeScript, JavaScript, Go, PHP, Swift**

Extracts: symbols, imports, function signatures, doc comments, class/struct hierarchies.

### Tier 3: Regex (minimal fallback)

For unknown languages or when tree-sitter fails. Line-based pattern matching for function definitions. No structural analysis.

**Feature-level fallback (not just per-language):**

```
Definition:    LSP → tree-sitter symbol lookup → grep
References:    LSP → tree-sitter import analysis → grep
Type errors:   LSP → (omitted, no fallback)
Symbols:       LSP → tree-sitter → regex
Call hierarchy: LSP → tree-sitter import graph → unavailable
```

---

## Session State Machine

```
Session Start ──► IDLE
                    │
                    │ project_map / smart_read
                    ▼
              ◄── ANALYZING ──►
                    │
                    │ smart_edit
                    ▼
                  EDITING ──── smart_read (after edit) ──► ANALYZING
                    │
                    │ smart_run
                    ▼
                VERIFYING
                    │  │
                    │  └── test fails ──► ANALYZING
                    │
                    │ tests pass
                    ▼
                 COMPLETE ──► commit / report
```

State transitions are **informational, not enforced** — a `smart_edit` in IDLE state is allowed (triggers a warning: "No prior read for this file"). The state machine improves hook decisions and `session_status` output.

| State | session_status shows | Hook behavior |
|-------|---------------------|---------------|
| IDLE | "No activity" | — |
| ANALYZING | Files read, symbols seen | — |
| EDITING | Edits + impact | TDD warning if no test ran |
| VERIFYING | Test results, error loop detection | Error loop tracking |
| COMPACTING | Summary being persisted | PreCompact/PostCompact |
| COMPLETE | Verification checklist | Stop hook |

---

## Compaction Recovery

When Claude Code runs `/compact` or auto-compact triggers at 95% context capacity, all working context is normally lost. codeaware-mcp recovers it via a 5-step pipeline:

**1. CAPTURE** (PostToolUse hook, running continuously)
Every tool result is indexed as a session event in SQLite FTS5:
```
{ tool, path, symbols_seen, edit_summary, test_result, timestamp }
```

**2. SNAPSHOT** (PreCompact hook)
Current session state is written as a JSON snapshot:
```
{ seen_files, edits_made, error_signatures, current_task, state }
```

**3. COMPACT** (Claude Code runs compaction)
Conversation history is summarized. All prior tool results are gone from context.

**4. RESTORE** (PostCompact hook)
Snapshot is loaded. Seen-files are marked "pre-compact" (hashes remain valid). `session_status` now returns:
```
"After compaction. 3 files read, 1 edit applied, last test: 47/48 passed.
 Active task: Fix auth bug in src/auth.rs"
```

**5. CONTINUE** (on `--continue` / session resume)
FTS5 `MATCH` query with BM25 ranking. Only events above a relevance threshold are injected. No full replay of the previous session.

The FTS5 virtual table indexes: `tool_name`, `file_path`, `symbols`, `summary`, `error_signature`. BM25 ranking ensures the most relevant prior context appears first.

---

## Security

All security checks run on every tool call, before results are returned to Claude.

### Path traversal protection

All paths are normalized (resolving `../`, symlinks, and relative components) and validated against the project root. Violations return `E_PATH_TRAVERSAL` immediately.

### 14 secret scanner patterns

Output is scanned before being returned. Matches are redacted to `[REDACTED:pattern_name]`:

| Pattern name | Detects |
|-------------|---------|
| `aws_key` | `AKIA[0-9A-Z]{16}` |
| `aws_secret` | AWS secret access key format |
| `github_token` | `ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_` prefixes |
| `openai_key` | `sk-[A-Za-z0-9]{20,}` |
| `anthropic_key` | `sk-ant-[A-Za-z0-9_-]{40,}` |
| `stripe_live` | `sk_live_[A-Za-z0-9]{24,}` |
| `stripe_test` | `sk_test_[A-Za-z0-9]{24,}` |
| `jwt` | Three-part base64url JWT structure |
| `twilio_sid` | `AC` + 32 hex characters |
| `private_key` | PEM block header |
| `password_url` | Credentials embedded in URLs |
| `api_key` | Generic `api_key = "..."` patterns |
| `token` | Generic `token = "..."` patterns |
| `secret` | Generic `secret = "..."` / `password = "..."` patterns |

The scanner checks up to 100KB per output. Larger outputs are truncated before scanning.

### Deny list

Per-project configurable lists of files and directories that cannot be read or modified, regardless of what Claude requests.

---

## Plugin Distribution

codeaware-mcp supports two deployment modes.

### Embedded Mode

Check in directly to your repository. Every developer gets the same configuration.

```
my-project/
├── .claude/
│   ├── settings.json
│   ├── rules/
│   │   ├── token-efficiency.md
│   │   └── security-policy.md
│   ├── skills/
│   │   ├── analyze/SKILL.md       ← /analyze
│   │   ├── fix/SKILL.md           ← /fix
│   │   ├── review/SKILL.md        ← /review
│   │   ├── project-map/SKILL.md   ← /project-map
│   │   ├── smart-read/SKILL.md    ← preloaded in agents
│   │   ├── smart-edit/SKILL.md    ← preloaded in agents
│   │   ├── smart-run/SKILL.md     ← preloaded in agents
│   │   └── gotchas/SKILL.md       ← preloaded in agents
│   └── agents/
│       ├── code-analyzer.md
│       ├── bug-fixer.md
│       └── code-reviewer.md
├── .mcp.json
├── .codeaware.toml
└── CLAUDE.md
```

### Plugin Mode

Install once, use across all projects. Skills get a namespaced invocation (`/codeaware:analyze`).

```
codeaware-plugin/
├── .claude-plugin/
│   └── plugin.json          ← only plugin.json lives here
├── skills/
│   ├── analyze/SKILL.md
│   ├── fix/SKILL.md
│   ├── review/SKILL.md
│   ├── project-map/SKILL.md
│   ├── smart-read/SKILL.md
│   ├── smart-edit/SKILL.md
│   ├── smart-run/SKILL.md
│   └── gotchas/SKILL.md
├── agents/
│   ├── code-analyzer.md
│   ├── bug-fixer.md
│   └── code-reviewer.md
├── hooks/
│   └── hooks.json
└── .mcp.json
```

`plugin.json`:
```json
{
  "name": "codeaware",
  "version": "1.0.0",
  "description": "Context-efficient orchestration and compression layer for Claude Code",
  "author": "mhmtbsbyndr",
  "license": "MIT"
}
```

**Scope priority** (when project and plugin define a skill with the same name):
- `/analyze` → project skill
- `/codeaware:analyze` → plugin skill

Plugin-shipped agents cannot include `hooks`, `mcpServers`, or `permissionMode` in their frontmatter.

---

## Skills and Agents

### User-invocable skills

| Skill | Command | What it does |
|-------|---------|-------------|
| Analyze | `/analyze` | Maps project structure and current task with minimal tokens. First step for unfamiliar code areas. |
| Fix | `/fix` | Fixes a bug: focused read of relevant files, compressed test output, TDD loop. |
| Review | `/review` | Reviews code changes in isolated agent context. No edits. |
| Project map | `/project-map` | Generates compressed project overview via `project_map` tool. |

All skills use `effort: high` and `context: fork` — they run in an isolated subagent context, keeping the main context clean.

### Preloaded skills (available to all agents)

`smart-read`, `smart-edit`, `smart-run`, `gotchas`

The `gotchas` skill contains known pitfalls and anti-patterns when using codeaware-mcp (e.g., "don't use skeleton mode for config files", "always pass expected_hash to smart_edit").

### Agents

| Agent | Model | Tools | Purpose |
|-------|-------|-------|---------|
| `code-analyzer` | Haiku (fast) | Read-only | Structural analysis, dependency mapping |
| `bug-fixer` | Sonnet | Full access | TDD-first bug fixing |
| `code-reviewer` | Sonnet | Read-only | Code review, no modifications |

All agents have an `initialPrompt` that auto-submits on the first turn, ensuring they start working immediately without a manual prompt.

---

## Hooks

Hooks run `codeaware-mcp hook <event>` and receive the event payload on stdin.

| Event | Trigger | What codeaware-mcp does |
|-------|---------|------------------------|
| `PostToolUse` | After every successful tool call | Index result in FTS5, scan output for secrets, update session state |
| `PostToolUseFailure` | After a tool error | Record error signature, increment pattern count, suggest fix if recurring |
| `PreCompact` | Before `/compact` or auto-compact | Write session snapshot to SQLite |
| `PostCompact` | After compaction completes | Load snapshot, mark seen-files as pre-compact |
| `SubagentStop` | When a subagent finishes | Merge subagent session data into parent session |
| `Stop` | When Claude Code session ends | Persist session summary, write file access patterns |

hooks.json:
```json
{
  "hooks": [
    { "event": "PostToolUse",        "command": "codeaware-mcp hook PostToolUse" },
    { "event": "PostToolUseFailure", "command": "codeaware-mcp hook PostToolUseFailure" },
    { "event": "PreCompact",         "command": "codeaware-mcp hook PreCompact" },
    { "event": "PostCompact",        "command": "codeaware-mcp hook PostCompact" },
    { "event": "SubagentStop",       "command": "codeaware-mcp hook SubagentStop" },
    { "event": "Stop",               "command": "codeaware-mcp hook Stop" }
  ]
}
```

---

## Configuration

Full `.codeaware.toml` reference:

```toml
[project]
name = "my-project"
root = "."

[intelligence]
prefer_lsp = false           # true = use LSP if available, false = tree-sitter only
fallback_to_regex = true     # fallback if tree-sitter fails

[compression]
enabled = true

[routing.defaults]
skeleton_threshold_loc = 50        # files > 50 LOC → smart_read
full_read_threshold_loc = 100      # files > 100 LOC → skeleton instead of full
test_output_always_compress = true # always use smart_run for test runners
config_files_never_compress = true # YAML/TOML/JSON → native Read

[session]
db_path = "~/.local/share/codeaware"
max_events = 10000

[enforcement]
scan_secrets = true
deny_read   = [".env", ".env.*", "secrets/**", "*.pem", "*.key"]
deny_edit   = ["*.generated.*", "*.pb.go", "Cargo.lock", "package-lock.json"]
deny_run    = ["rm -rf", "curl | sh", "wget | bash", "sudo *"]

[languages.rust]
lsp = "rust-analyzer"

[languages.typescript]
lsp = "typescript-language-server --stdio"

[languages.python]
lsp = "pylsp"
```

---

## Install

**Requirements:** Rust 1.75+, macOS/Linux

```bash
git clone https://github.com/mhmtbsbyndr/codeaware-mcp
cd codeaware-mcp
cargo build --release
cp target/release/codeaware-mcp /usr/local/bin/
```

Or build and install in one step:
```bash
cargo install --path .
```

### Add to a project

1. Create `.mcp.json` in your project root:
```json
{
  "mcpServers": {
    "codeaware": {
      "command": "codeaware-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

2. Add to `CLAUDE.md`:
```markdown
<important if="reading or editing files">
Prefer smart_read for files > 50 LOC, smart_run for tests/builds/linting,
smart_edit when impact analysis is needed.
Standard Read/Bash stays allowed for small files and simple commands.
</important>

<important if="starting a task">
Start with /analyze or project_map for unfamiliar code areas.
</important>
```

3. Optionally add `.codeaware.toml` for project-specific settings.

---

## Platform Support

| Platform | MCP Tools | Hooks | Skills/Agents |
|----------|:---------:|:-----:|:-------------:|
| Claude Code | ✅ | ✅ all 6 events | ✅ |
| Gemini CLI | ✅ | partial | — |
| VS Code Copilot | ✅ | partial | — |
| Cursor | ✅ | partial | — |
| OpenCode | ✅ | partial | — |

MCP tools (smart_read, smart_edit, smart_run, etc.) work on any platform with stdio MCP support. Skills, agents, and hooks are Claude Code-specific and are silently ignored on other platforms.

---

## Architecture

```
Claude Code
    │
    │  JSON-RPC 2.0 over stdio
    ▼
┌──────────────────────────────────────────────────────┐
│  codeaware-mcp (Rust)                                │
│                                                      │
│  McpServer                                           │
│  ├─ tools/project_map.rs                             │
│  ├─ tools/smart_read.rs                              │
│  ├─ tools/smart_edit.rs                              │
│  ├─ tools/smart_run.rs                               │
│  ├─ tools/workspace_state.rs                         │
│  ├─ tools/session_status.rs                          │
│  └─ tools/validate_config.rs                         │
│                                                      │
│  intelligence/                                       │
│  ├─ lsp_client.rs        (LSP, 2s timeout)           │
│  ├─ tree_sitter_provider.rs  (7 languages)           │
│  └─ regex_fallback.rs    (pattern-based)             │
│                                                      │
│  security/                                           │
│  ├─ path_resolver.rs     (traversal protection)      │
│  ├─ secret_scanner.rs    (14 patterns, 100KB limit)  │
│  └─ deny_list.rs         (configurable blocks)       │
│                                                      │
│  session/                                            │
│  ├─ state.rs             (state machine)             │
│  ├─ persistence.rs       (SQLite WAL + FTS5)         │
│  ├─ workspace_state.rs   (5 typed slots)             │
│  └─ patterns.rs          (error signature tracking)  │
│                                                      │
│  compressor/                                         │
│  ├─ test_output.rs       (test runners)              │
│  ├─ compiler_output.rs   (rustc, gcc, tsc)           │
│  ├─ linter_output.rs     (clippy, eslint)            │
│  ├─ git_output.rs        (diff, log, status)         │
│  └─ generic.rs           (fallback truncation)       │
└──────────────────────────────────────────────────────┘
         │
         ▼
  SQLite (WAL mode)
  ├─ sessions
  ├─ file_access_patterns
  ├─ error_signatures
  ├─ session_events_content
  └─ session_events (FTS5 virtual table)
```

Single static binary. SQLite bundled (no external dependency). tree-sitter grammars compiled in. ~8MB binary size.

---

## Tests

```bash
cargo test        # 175 tests across 30 test files, ~1s
cargo clippy      # 0 warnings
```

Test coverage: MCP envelope, all 7 tools, path traversal, secret scanner (all 14 patterns), tree-sitter symbol extraction (all 7 languages), FTS5 round-trip, workspace state slots, config validation findings, acceptance matrix T01–T17 + N01–N12.

---

## License

MIT
