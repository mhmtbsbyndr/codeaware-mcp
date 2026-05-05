# Token-Saving Gap Analysis

This document captures the strongest ideas from other token-saving MCP/code-awareness projects and translates them into concrete upgrade directions for `codeaware-mcp`.

`codeaware-mcp` is already strong in code/file/git/terminal compression. The main opportunity is to expand from local per-call compression into a broader code-intelligence and token-economy platform.

## Current Strength

`codeaware-mcp` already focuses on:

- AST-aware file compression;
- focused file reads;
- git-aware diffs and blame;
- command output compression;
- structured MCP responses;
- session state and compaction recovery;
- smart edits with hash guards;
- security-aware command/file handling.

## Missing or Weakly Represented Areas

## 1. Structured Code Graph and Symbol Indexing

Other token-saving/code-understanding systems often expose a real code graph instead of forcing agents to repeatedly inspect files.

### Gap

`codeaware-mcp` has AST extraction and file-level intelligence, but should expose more whole-repository graph APIs.

### Target capabilities

- full repository symbol index;
- `get_references`;
- `get_callers`;
- `get_callees`;
- `get_outline`;
- `find_usages`;
- complexity metrics;
- critical-file ranking;
- repo map with symbols, imports, dependency edges, and risk scores.

### Candidate MCP tools

```text
query_symbols
get_references
get_callers
get_callees
get_outline
find_usages
complexity_map
critical_files
```

### Implementation notes

- Use tree-sitter for broad language coverage.
- Add LSP integration as optional higher-trust enrichment.
- Store symbol/index data in SQLite.
- Track `trust = exact | structural | heuristic | raw` per edge.

## 2. Persistent Memory Engine

Other tools use SQLite, FTS, and sometimes vectors to persist decisions, conventions, bugfixes, and observations across sessions.

### Gap

`codeaware-mcp` already documents memory features, but the next step should be a stronger persistent memory engine with explicit resume workflows, TTL, ranking, and contradiction handling.

### Target capabilities

- persistent SQLite memory DB;
- FTS5 search;
- optional vector search adapter;
- smart resume by task/repo/branch;
- memory TTL and decay;
- validity ranking;
- contradiction detection;
- delta-only context reinjection;
- memory types: decision, convention, bugfix, discovery, risk, test-failure, architecture-note.

### Candidate MCP tools

```text
write_memory
read_memory
search_memories
smart_resume
memory_decay_report
memory_contradictions
memory_promote
memory_archive
```

### Implementation notes

- Keep memory local-first.
- Never upload source snippets by default.
- Store file paths, symbols, facts, confidence, source trace, and branch.
- Separate short-lived session state from durable project memory.

## 3. Editor / LSP / Browser Token Optimization

Some token-saving systems integrate with VS Code LSP or Chrome DevTools Protocol and return structured editor/browser data instead of raw dumps.

### Gap

`codeaware-mcp` is strong in filesystem, git, and terminal workflows, but has limited editor/browser awareness.

### Target capabilities

- LSP hover;
- definitions;
- references;
- diagnostics;
- rename previews;
- symbol search;
- editor active-file awareness;
- optional CDP snapshot compression for web-app debugging;
- DOM summary instead of full HTML;
- console/network/error summary instead of raw browser logs.

### Candidate MCP tools

```text
lsp_hover
lsp_definitions
lsp_references
lsp_diagnostics
lsp_rename_preview
browser_snapshot_summary
browser_console_summary
browser_network_summary
```

### Implementation notes

- Keep LSP/CDP optional.
- Degrade cleanly when no editor/browser bridge exists.
- Use the same trust envelope as normal code-intelligence tools.

## 4. Tool Schema and Meta-Tool Optimization

Some projects reduce MCP token overhead by exposing only meta-tools and lazily loading detailed schemas.

### Gap

`codeaware-mcp` saves tokens mainly through content compression, not tool-schema compression.

### Target capabilities

- compact tool registry;
- `list_capabilities` instead of large static descriptions;
- `get_tool_schema` loaded on demand;
- tool aliases for common workflows;
- task-profile-based tool exposure;
- schema minification mode.

### Candidate MCP tools

```text
list_capabilities
get_tool_schema
select_tool_profile
schema_budget_report
```

### Implementation notes

- Keep normal MCP compatibility.
- Add a low-token mode for clients that support dynamic schema discovery.
- Group tools into profiles: `read`, `edit`, `git`, `memory`, `ci`, `browser`, `research`.

## 5. Protocol-Level and Orchestration Features

Some systems support batching, streamable HTTP, discovery, or smart routing between multiple MCP servers.

### Gap

`codeaware-mcp` is mostly a single-purpose local MCP server.

### Target capabilities

- batch tool calls;
- streamable HTTP transport as optional alternative to stdio;
- local MCP router mode;
- multi-server discovery;
- route code tasks to codeaware, GitHub tasks to GitHub MCP, browser tasks to browser MCP;
- profile-based routing.

### Candidate MCP tools

```text
batch_call
route_task
discover_mcp_servers
mcp_server_health
```

### Implementation notes

- Do not replace dedicated MCP servers.
- Act as a routing/compression layer when explicitly enabled.
- Keep stdio as the default, simplest mode.

## 6. Monitoring, Benchmarking, and ROI Layer

Other token-saving tools are strong because they measure token savings and show ROI clearly.

### Gap

`codeaware-mcp` should make token savings visible and auditable.

### Target capabilities

- per-tool token estimates;
- raw-vs-compressed token deltas;
- session-level savings;
- project-level savings;
- estimated cost savings;
- saved context-window percentage;
- time saved estimates;
- benchmark fixtures;
- README benchmark table generated from real test fixtures.

### Candidate MCP tools

```text
token_stats
token_savings_report
benchmark_compression
roi_report
session_budget
```

### Implementation notes

- Use deterministic token approximation if exact tokenizer is unavailable.
- Record input/output byte size, estimated token size, compression ratio, and category.
- Keep benchmark fixtures in `benches/fixtures`.

## 7. Suggested Priority Order

### Phase 1 — Measurement First

Build the ROI layer before adding many new tools.

Deliver:

- `token_stats`
- `token_savings_report`
- basic benchmark fixtures
- README metrics section

Why: this makes every later improvement measurable.

### Phase 2 — Persistent Index

Deliver:

- SQLite symbol index
- `query_symbols`
- `get_outline`
- `get_references`
- incremental indexing by file hash

Why: this removes repeated full-file reads.

### Phase 3 — Smart Resume Memory

Deliver:

- `smart_resume`
- durable memory search
- branch/task-aware memory records
- TTL/decay

Why: this solves compaction and cross-session loss.

### Phase 4 — LSP Bridge

Deliver:

- diagnostics
- definitions
- references
- hover

Why: this gives exact editor-aware context without file dumps.

### Phase 5 — Schema and Router Optimization

Deliver:

- dynamic tool schema loading
- tool profiles
- batch call
- optional router mode

Why: this reduces overhead once the tool surface grows.

## Comparison Table

| Category | Current codeaware-mcp | Upgrade target |
|---|---|---|
| Code graph | File-level AST and local intelligence | Persistent repo-wide symbol graph |
| Memory | Session and documented memory flows | Durable memory DB with resume, TTL, contradiction detection |
| Editor/LSP | Limited or optional | Structured hover/refs/diagnostics/definition tools |
| Browser/CDP | Not core | Optional compressed browser snapshots/logs |
| Tool schema overhead | Normal MCP tool descriptions | Lazy schema loading and tool profiles |
| Monitoring | Informal token-saving claim | Measured token stats and ROI reports |
| Orchestration | Single MCP server | Optional router/discovery/batch layer |

## Design Principle

Do not copy every feature from other projects. Absorb only the parts that reinforce the core identity:

> `codeaware-mcp` is a local-first, deterministic, evidence-based, token-saving code intelligence layer for AI coding agents.

Everything should preserve:

- local-first operation;
- deterministic JSON output;
- evidence over claims;
- trust levels;
- safe fallback paths;
- measurable token reduction;
- compatibility with Claude Code and other MCP clients.
