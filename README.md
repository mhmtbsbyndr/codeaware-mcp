# codeaware-mcp

A Rust MCP server that sits between Claude Code and your filesystem as a **compression and orchestration layer**. Every tool call returns structured, token-efficient output instead of raw content.

> **Target:** 70–95% token reduction depending on task type and file size (benchmark target, not a guarantee — varies by codebase and workflow).

---

## The Problem

Claude Code's built-in tools return raw content:

- `Read` on a 500-line file → 500 lines in context
- `Bash` running `cargo test` → hundreds of lines of output
- Re-reading the same file after an edit → full content again
- After `/compact` → everything you were working on is gone

Every token consumed is money spent and context window exhausted. On larger projects, Claude regularly hits context limits mid-task.

---

## How codeaware-mcp Solves This

Instead of raw content, every tool returns a **structured, compressed result**:

```
smart_read(file.rs, mode=skeleton)
→ { symbols: [...], imports: [...], loc: 420, stale: false, suggested_next: [...] }
  (not 420 lines of source)

smart_run("cargo test")
→ { status: "fail", failed: 2, passed: 41, failures: [{test: "...", message: "..."}] }
  (not 300 lines of test output)
```

The server tracks what it has already delivered. On second read of the same file, it only returns a diff. After `/compact`, FTS5-indexed session events let Claude recover exactly the context it needs via BM25 search.

---

## Comparison with Other MCP Servers

| Feature | [Context Mode](https://github.com/ContextMode) | [Token Optimizer](https://github.com/token-optimizer) | CC Token Saver | **codeaware-mcp** |
|---------|--------------|---------------|---------------|-------------------|
| File read compression | None | ~80% (chunking) | None | **90–95%** (AST skeleton/focused) |
| Test output compression | None | None | None | **~92%** (failures + code only) |
| Build output compression | None | None | None | **~93%** (errors only) |
| Session state after /compact | ✅ Full restore | Memory store | Nothing | ✅ **Full restore** |
| Code understanding | None | Signatures (regex) | None | **LSP + tree-sitter** (callers, types, impact) |
| Platforms | 5 | 3 | 1 | **5** |
| Edit safety | None | None | None | **Transactional** (hash, atomic, rollback) |
| Skills + Agents + Hooks | Partial | None | None | ✅ **Full orchestration** |

### What makes codeaware-mcp different

**AST-based compression, not chunking.** Other tools either return raw content or chunk files arbitrarily. codeaware-mcp uses tree-sitter to understand the code — it extracts symbols, caller chains, and import graphs, and delivers only what Claude needs for the current task.

**Impact analysis on edits.** `smart_edit` returns the list of callers affected by a change, the relevant tests to run, and a syntax validity check — before Claude decides whether to proceed. No other MCP server does this.

**Transactional edits.** Every edit takes an `expected_hash` of the current file state. If the file changed between read and edit (e.g., another tool ran), the edit is rejected atomically. No silent overwrites.

**Compaction recovery.** Every tool call is indexed as a session event in SQLite FTS5. After `/compact` or `--continue`, `session_status` retrieves the most relevant prior context via BM25 ranking — Claude picks up where it left off without re-reading everything.

**Trust levels on responses.** Each result includes an `intelligence_level` field: `lsp` / `tree-sitter` / `regex` / `raw`. Claude knows how reliable the code analysis is and can adjust its confidence accordingly.

---

## Tools

### `smart_read` — Context-aware file reading

Four modes, selected automatically or explicitly:

| Mode | When | Output |
|------|------|--------|
| `skeleton` | Files > 100 LOC (default for large files) | Symbols, imports, structure — no source |
| `focused` | With `focus: "function_name"` | Only the requested function + its direct callees |
| `full` | Small files or explicit | Full content, hash-tracked |
| `diff` | File already read this session | Only what changed since last read |

Returns: `{ symbols, imports, callers, relevant_tests, content?, stale, file_hash, suggested_next }`

### `smart_edit` — Edit with impact analysis

```json
{
  "path": "src/server.rs",
  "mode": "text",
  "old": "fn handle_request(",
  "new": "fn handle_request_v2(",
  "expected_hash": "a3f9..."
}
```

Returns before applying the edit:
- Affected callers (functions/files that call the changed symbol)
- Relevant test files
- Syntax validity check
- Hash of the new state

Rejects if `expected_hash` doesn't match current file state.

### `smart_run` — Compressed command output

Classifies commands into 8 categories and applies category-specific compression:

| Category | Compression strategy |
|----------|---------------------|
| `test_runner` | Failed tests only + failure messages |
| `compiler` | Errors + file:line references, warnings summarized |
| `linter` | Grouped by rule, count per file |
| `git` | Structured diff summary |
| `package_mgr` | Added/removed packages only |
| `formatter` | Changed file count only |
| `search` | Match count + top results |
| `generic` | Last N lines + exit code |

Returns: `{ exit_code, category, summary, errors: [...], compressed: true }`

### `project_map` — Compressed project overview

Returns a compressed representation of the project structure: entry points, module tree, file sizes, dependency graph edges — without reading every file.

Useful as first call when entering an unfamiliar codebase.

### `workspace_state` — Cross-tool session state

Five typed slots for sharing state between tool calls within a session:

| Slot | Schema | Purpose |
|------|--------|---------|
| `recent_targets` | `{ files: [...] }` | Files currently being worked on |
| `error_signatures` | `{ errors: [...] }` | Recurring error patterns with fix suggestions |
| `co_access_candidates` | `{ pairs: [...] }` | Files that tend to be edited together |
| `verification_state` | `{ status, checks: [...] }` | Current test/build pass/fail state |
| `active_task` | `{ ... }` | Arbitrary task context object |

On first read: returns full content. On subsequent reads: returns only what changed (`full: false`).

### `validate_config` — Structured config findings

Scans `.codeaware.toml` (or any config file) and returns structured findings:

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
  "categories": { "security": 7.0, "quality": 9.0, "efficiency": 8.5 }
}
```

Finding codes: `SEC-001/002/003`, `QUL-001/002/003`, `EFF-001/002/003`

### `session_status` — Session summary and compaction recovery

Returns current session state: files touched, error patterns seen, token estimates, and (after `/compact`) the most relevant prior context retrieved via FTS5 BM25 search.

---

## Security

Built-in enforcement that runs on every tool call:

**Path traversal protection.** All paths are normalized and validated against the project root. `../` traversal, symlink escapes, and absolute paths outside the project are rejected with `E_PATH_TRAVERSAL`.

**14 secret scanner patterns.** Tool output is scanned before being returned to Claude. Matches are redacted to `[REDACTED:pattern_name]`:

| Pattern | Example match |
|---------|--------------|
| `aws_key` | `AKIA...` |
| `github_token` | `ghp_...`, `gho_...` |
| `openai_key` | `sk-...` |
| `anthropic_key` | `sk-ant-...` |
| `stripe_live` | `sk_live_...` |
| `stripe_test` | `sk_test_...` |
| `jwt` | `eyJ...` |
| `aws_secret` | AWS secret access key pattern |
| `twilio_sid` | `AC` + 32 hex chars |
| `private_key` | PEM block header |
| `password_url` | Credentials in URLs |
| `api_key` | Generic `api_key=...` |
| `token` | Generic `token=...` |
| `secret` | Generic `secret=...` |

**Deny list.** Configurable per-project list of files/directories that cannot be read or modified.

---

## Code Intelligence

Three-tier fallback with explicit reliability reporting:

```
LSP (language server, if running)
  → tree-sitter (compiled grammar, always available)
    → regex (language-agnostic fallback)
```

Each result includes `intelligence_level: "lsp" | "tree-sitter" | "regex" | "raw"` so Claude knows what it's working with.

**Supported languages (tree-sitter):** Rust, Python, TypeScript, JavaScript, Go, PHP, Swift

**Extracted per symbol:**
- Name, kind (function/class/struct/method/etc.), line range
- Direct callers (functions that call this symbol)
- Relevant test files

---

## Session Persistence

SQLite database (WAL mode) per project, stored at `~/.local/share/codeaware/<project-hash>.db`.

Tables:
- `sessions` — session metadata, summaries, files touched
- `file_access_patterns` — per-file access counts, co-access pairs, read modes
- `error_signatures` — recurring error patterns with suggested fixes
- `session_events_content` + `session_events` (FTS5 virtual table) — all tool calls indexed for BM25 search

After `/compact`, `session_status` queries `session_events MATCH ?` with BM25 ranking to retrieve the most relevant prior context. Claude gets a focused recovery summary instead of nothing.

---

## Skills and Agents (Claude Code)

### User-invocable skills

| Skill | Usage |
|-------|-------|
| `/analyze` | Analyze project structure and current task with minimal tokens |
| `/fix` | Fix a bug with focused reading and compressed test output |
| `/review` | Review code changes in isolated context |
| `/project-map` | Generate compressed project overview |

### Preloaded skills (always available to agents)

`smart-read`, `smart-edit`, `smart-run`, `gotchas`

### Agents

| Agent | Model | Capabilities |
|-------|-------|-------------|
| `code-analyzer` | Haiku (fast) | Read-only, structural analysis |
| `bug-fixer` | Sonnet | TDD-first, full edit access |
| `code-reviewer` | Sonnet | Read-only, no edits |

### Hooks

| Event | Action |
|-------|--------|
| `PostToolUse` | Index tool result as session event, scan for secrets |
| `PostToolUseFailure` | Record error signature, increment pattern count |
| `PreCompact` | Snapshot workspace_state slots |
| `PostCompact` | Restore workspace_state from snapshot |
| `SubagentStop` | Merge subagent session data |
| `Stop` | Persist session summary |

---

## Platform Support

| Platform | Tools | Hooks | Skills/Agents |
|----------|-------|-------|---------------|
| Claude Code | ✅ | ✅ All 6 events | ✅ |
| Gemini CLI | ✅ | Partial | ❌ |
| VS Code Copilot | ✅ | Partial | ❌ |
| Cursor | ✅ | Partial | ❌ |
| OpenCode | ✅ | Partial | ❌ |

MCP tools work on any platform with stdio MCP support. Skills, agents, and hooks are Claude Code-specific.

---

## Install

**Requirements:** Rust 1.75+

```bash
git clone https://github.com/mhmtbsbyndr/codeaware-mcp
cd codeaware-mcp
cargo build --release
cp target/release/codeaware-mcp /usr/local/bin/
```

### Configure in your project

`.mcp.json`:
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

`CLAUDE.md` (add to your project):
```markdown
<important if="reading or editing files">
Prefer smart_read for files > 50 LOC, smart_run for tests/builds/linting,
smart_edit when impact analysis is needed.
Standard Read/Bash is fine for small files and simple commands.
</important>
```

### Optional: `.codeaware.toml`

```toml
[project]
name = "my-project"
root = "."

[enforcement]
scan_secrets = true
deny_read = [".env", "secrets/**", "*.pem"]
deny_edit = ["*.generated.*", "Cargo.lock"]
deny_run = ["rm -rf", "curl | sh"]

[intelligence]
prefer_lsp = false       # set true if you have a language server running
fallback_to_regex = true

[session]
db_path = "~/.local/share/codeaware"
max_events = 10000
```

---

## Architecture

```
Claude Code
    │
    │  JSON-RPC over stdio
    ▼
┌─────────────────────────────────────┐
│           codeaware-mcp             │
│                                     │
│  McpServer (tool registry + router) │
│       │                             │
│  ┌────┴──────────────────────┐      │
│  │ Tools                     │      │
│  │  smart_read               │      │
│  │  smart_edit               │      │
│  │  smart_run                │      │
│  │  project_map              │      │
│  │  workspace_state          │      │
│  │  session_status           │      │
│  │  validate_config          │      │
│  └───────────────────────────┘      │
│       │                             │
│  ┌────┴──────────────────────┐      │
│  │ Intelligence              │      │
│  │  LSP → tree-sitter → regex│      │
│  │  (Rust/Py/TS/JS/Go/PHP/Swift)    │
│  └───────────────────────────┘      │
│       │                             │
│  ┌────┴──────────────────────┐      │
│  │ Security                  │      │
│  │  PathResolver (traversal) │      │
│  │  SecretScanner (14 pats)  │      │
│  │  DenyList                 │      │
│  └───────────────────────────┘      │
│       │                             │
│  ┌────┴──────────────────────┐      │
│  │ Session (SQLite WAL+FTS5) │      │
│  │  SessionDb                │      │
│  │  SessionState             │      │
│  │  WorkspaceSlots           │      │
│  └───────────────────────────┘      │
└─────────────────────────────────────┘
```

Built in Rust. Zero runtime dependencies beyond the binary (SQLite bundled, tree-sitter grammars compiled in). Single static binary, ~8MB.

---

## Tests

```bash
cargo test        # 175 tests, ~1s
cargo clippy      # 0 warnings
```

---

## License

MIT
