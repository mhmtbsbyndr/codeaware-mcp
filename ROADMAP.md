# codeaware-mcp Roadmap

This roadmap turns `codeaware-mcp` from a local token-saving MCP server into a deeper **code-aware intelligence layer** inspired by Open Aware, Samsung CAS, Aurora-style hot-code memory, and build-aware metadata extraction.

## Vision

`codeaware-mcp` should become the local-first bridge between AI coding agents and real repositories:

- compress source code, command output, diffs, and test results before they reach the agent;
- understand project structure, symbols, dependencies, build graphs, and risk hotspots;
- persist cross-session memory without leaking code to third-party services;
- expose everything through stable MCP tools with deterministic JSON responses;
- support Claude Code, Codex-style agents, local LLMs, and CI workflows.

## v2 Baseline

Already documented in the README:

- Rust single-binary MCP server
- AST/tree-sitter based smart reads
- smart edits with hash guards
- command output classification
- SQLite FTS5 memory
- git intelligence
- test coverage mapping
- hooks, skills, agents, dashboard

## v3 Upgrade Tracks

### 1. Repo Intelligence Index

Goal: build a persistent, queryable project index that survives sessions.

Planned capabilities:

- per-file symbol graph
- import/export graph
- call graph with confidence levels
- dependency graph from package manager files
- test-to-source mapping
- code ownership hints from git history
- hotspot ranking by churn, complexity, failing tests, and recent edits

New internal tables:

- `files`
- `symbols`
- `references`
- `imports`
- `call_edges`
- `test_edges`
- `git_churn`
- `hotspots`

Candidate MCP tools:

- `index_project`
- `query_symbols`
- `explain_symbol`
- `dependency_graph`
- `hotspots`

### 2. Build-Aware Metadata Layer

Inspired by Samsung CAS: the server should know which files are actually compiled, tested, bundled, or ignored.

Planned capabilities:

- parse build/test commands from common project files
- capture build invocation metadata
- map source files to build targets where possible
- detect generated files and vendor directories
- separate runtime dependencies from dev dependencies
- explain why a file is relevant to a build/test command

Candidate MCP tools:

- `build_map`
- `target_files`
- `why_file_in_build`
- `affected_targets`

Supported ecosystems first:

- Rust/Cargo
- Node/npm/pnpm/yarn
- Python/pytest/poetry
- PHP/Composer/Laravel
- Go modules

### 3. Deep Code Research Mode

Inspired by qodo-ai/open-aware: give agents a high-level research endpoint instead of forcing many low-level reads.

Candidate tool: `deep_research`

Input:

```json
{
  "question": "How does authentication work?",
  "scope": "src/auth",
  "budget": "medium",
  "include_evidence": true
}
```

Output:

```json
{
  "answer": "Authentication is handled by AuthManager...",
  "evidence": [
    { "file": "src/auth.rs", "symbol": "AuthManager", "lines": "42-120", "trust": "structural" }
  ],
  "open_questions": ["Refresh token expiry is not obvious from static analysis"],
  "suggested_next": ["query_symbols AuthManager", "smart_read src/auth.rs focus=refresh_token"]
}
```

Design rules:

- every answer must include file/symbol evidence;
- no unverifiable claims unless marked as inference;
- keep raw source snippets short;
- prefer structured evidence over prose.

### 4. Hot/Warm/Cool Code Memory

Inspired by Aurora-style code-aware memory.

Classification:

- **hot**: recently edited, frequently read, failing, or high-churn code;
- **warm**: stable but often referenced code;
- **cool**: rarely touched support code;
- **frozen**: generated/vendor/archived code.

Use cases:

- prioritize project maps;
- reduce token usage by avoiding cool code unless needed;
- warn before editing hot high-risk modules;
- suggest tests based on hot path impact.

Candidate MCP tools:

- `thermal_map`
- `risk_report`
- `suggest_context_budget`

### 5. Multi-Repo Workspace Awareness

Many real tasks span API, frontend, SDK, and docs repos.

Planned capabilities:

- workspace manifest with multiple roots;
- cross-repo symbol search;
- cross-repo memory search;
- dependency edges between repos;
- shared task state across repositories.

Candidate config:

```toml
[workspace]
name = "my-platform"
roots = ["../api", "../frontend", "../sdk", "../docs"]
```

Candidate MCP tools:

- `workspace_map`
- `cross_repo_search`
- `cross_repo_impact`

### 6. CI / PR Intelligence

Make `codeaware-mcp` useful outside local Claude Code sessions.

Planned capabilities:

- summarize PR diffs;
- map PR changes to affected symbols/tests;
- generate review checklist;
- compress CI logs;
- store failed CI signatures in memory;
- suggest minimal test commands.

Candidate tools:

- `pr_context`
- `ci_failure_summary`
- `review_plan`
- `minimal_tests`

### 7. Governance and Safety Hardening

For enterprise and local-first usage, safety must be explicit.

Planned controls:

- deny-list for dangerous commands;
- secret redaction in command output;
- path traversal and symlink escape prevention;
- audit trace IDs on every tool call;
- configurable retention policy;
- offline-only mode;
- optional allow-list for writable paths;
- risk scoring before destructive edits.

Candidate tools:

- `security_audit`
- `policy_check`
- `trace_lookup`

## Proposed Milestones

### Milestone 1 — Index Core

- persistent symbol index
- file metadata table
- incremental re-indexing by file hash
- `index_project`
- `query_symbols`

### Milestone 2 — Research Tools

- `deep_research`
- `explain_symbol`
- evidence model
- confidence/trust annotations

### Milestone 3 — Build Awareness

- Cargo, npm/pnpm/yarn, Composer, Go module detection
- build target map
- affected target calculation

### Milestone 4 — Hotspot and Risk Engine

- git churn importer
- hot/warm/cool classification
- risk report before edit/refactor

### Milestone 5 — CI / PR Mode

- PR diff summarizer
- minimal test selection
- CI failure memory
- review plan generation

## Non-Goals

- replacing GitHub MCP;
- replacing full LSP servers;
- uploading private code to a hosted index by default;
- acting as an autonomous write agent without explicit user approval;
- hiding uncertainty from the coding agent.

## Success Metrics

- reduce repeated code-read tokens by 80%+ on medium repositories;
- answer architecture questions with cited file/symbol evidence;
- identify impacted tests for common changes;
- survive compaction without losing task state;
- keep all code and memory local by default;
- produce deterministic MCP responses suitable for CI and agent workflows.
