# 🧠 codeaware-mcp

> 🇩🇪 Deutsche Version: [README.de.md](README.de.md)

<p align="center">
  <strong>Local-first AI Code Intelligence, Token Compression, Progressive Memory, Semantic Repository Runtime & Quality Layer for MCP Agents</strong>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?style=for-the-badge&logo=rust" />
  <img alt="MCP" src="https://img.shields.io/badge/MCP-JSON--RPC-blue?style=for-the-badge" />
  <img alt="Local First" src="https://img.shields.io/badge/Local--First-Yes-success?style=for-the-badge" />
  <img alt="Status" src="https://img.shields.io/badge/Status-v4%20Semantic%20Runtime-purple?style=for-the-badge" />
</p>

---

## 🚀 What is codeaware-mcp?

`codeaware-mcp` is a **local-first MCP runtime** for AI coding agents such as Claude Code, Codex-style agents, Cursor/OpenCode-style workflows, Gemini CLI-style agents, and local LLM workflows.

It sits between the agent and your repository and returns **compressed, structured, evidence-based code intelligence** instead of raw files, noisy terminal output, repeated diffs, and unmeasured token usage.

The current project is best described as:

> **A local-first Persistent Code Intelligence Runtime with stable compression foundations and a v4 semantic context kernel for bounded AI coding agents.**

The core idea is simple:

```text
The LLM should not own repository context.
CodeAware should.
```

---

## 🧠 Why this exists

Modern AI coding tools are powerful, but many of them still work by repeatedly loading repository text into an LLM context window.

That creates five problems:

1. **Token burn** — the same files are read again and again.
2. **Context drift** — the model forgets why files matter after compaction.
3. **Weak traceability** — it is hard to know why a file was selected.
4. **Poor task boundaries** — autonomous agents over-explore instead of executing a bounded patch.
5. **No persistent repository semantics** — every session rebuilds understanding from raw text.

CodeAware v4 attacks the root problem:

```text
Do not make the LLM rediscover the repository.
Compile the repository into reusable semantic intelligence first.
```

---

## 🧠 CodeAware v4 Kernel

CodeAware v4 adds a persistent semantic repository layer designed to reduce AI coding token waste, uncontrolled repo scans, and repeated context rehydration.

### v4 execution model

```text
Repository
→ Discovery
→ AST Parsing
→ Semantic Extraction
→ SemanticIndex
→ SemanticContextAssembler
→ ContextPackage
→ Agent
→ Trace
→ Recovery
→ Architecture Memory
→ Semantic Routing
```

### Implemented v4 runtime modules

```text
src/v4/
  architecture_memory.rs
  budget.rs
  cache.rs
  cache_invalidation.rs
  call_graph.rs
  context.rs
  context_items.rs
  contracts.rs
  discovery.rs
  errors.rs
  impact.rs
  import_graph.rs
  index_builder.rs
  language_support.rs
  precision.rs
  ranking.rs
  recovery.rs
  retrieval.rs
  semantic_context.rs
  semantic_index.rs
  semantic_router.rs
  semantic_tools.rs
  storage.rs
  summaries.rs
  symbols.rs
  tests_graph.rs
  tokens.rs
  tools.rs
  trace.rs
```

### v4 capabilities

| Capability | Status |
|---|---|
| Task contracts | Implemented |
| Budget engine | Implemented |
| Candidate discovery | Implemented |
| Ranking | Implemented |
| Summary-first fallback | Implemented |
| Token estimation | Implemented |
| Context packages | Implemented |
| JSONL trace persistence | Implemented |
| tree-sitter Rust AST parsing | Implemented |
| Symbol extraction | Implemented |
| Import graph | Implemented |
| Call graph foundation | Implemented |
| Test graph foundation | Implemented |
| Impact analysis foundation | Implemented |
| SemanticIndex | Implemented |
| SemanticContextAssembler | Implemented |
| semantic-first `get_task_context` | Implemented |
| Semantic tools: find_symbol/find_callers/find_tests/diff_impact | Implemented and wired into MCP dispatch |
| Architecture memory | Implemented foundation |
| Decision memory | Implemented foundation |
| Semantic recovery | Implemented foundation |
| Semantic router | Implemented foundation |
| Cache invalidation | Implemented foundation |
| Multi-language detection | Implemented foundation |
| Precision metrics | Implemented foundation |

---

## 🔌 v4 MCP tools

The following v4 tools are wired into the MCP `tools/call` dispatcher:

```text
codeaware.get_task_context
codeaware.find_symbol
codeaware.find_callers
codeaware.find_tests
codeaware.diff_impact
```

### `codeaware.get_task_context`

Builds a bounded semantic context package for an AI coding task.

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "codeaware.get_task_context",
    "arguments": {
      "repo_root": "/workspace/project",
      "goal": "Refactor semantic context assembly"
    }
  }
}
```

### `codeaware.find_symbol`

Finds symbols from the semantic index.

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "tools/call",
  "params": {
    "name": "codeaware.find_symbol",
    "arguments": {
      "repo_root": "/workspace/project",
      "query": "ContextPackage"
    }
  }
}
```

### `codeaware.find_callers`

Finds callers of a symbol.

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "tools/call",
  "params": {
    "name": "codeaware.find_callers",
    "arguments": {
      "repo_root": "/workspace/project",
      "symbol": "build_context"
    }
  }
}
```

### `codeaware.find_tests`

Finds tests related to a symbol.

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "tools/call",
  "params": {
    "name": "codeaware.find_tests",
    "arguments": {
      "repo_root": "/workspace/project",
      "symbol": "ContextPackage"
    }
  }
}
```

### `codeaware.diff_impact`

Estimates semantic impact for a changed file.

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "tools/call",
  "params": {
    "name": "codeaware.diff_impact",
    "arguments": {
      "repo_root": "/workspace/project",
      "changed_path": "src/v4/tools.rs"
    }
  }
}
```

---

## ⚖️ Comparison with AI coding tools

CodeAware is not trying to replace AI coding agents. It is the **semantic context layer beneath them**.

| Tool | Primary role | Strength | Weakness CodeAware addresses |
|---|---|---|---|
| Claude Code | Premium coding agent | Strong reasoning and patch execution | Can burn context/tokens on repo exploration |
| Cursor | AI IDE | Fast inline coding and editor UX | Usage can rise with large contexts and repeated scans |
| Gemini CLI | Budget-friendly terminal agent | Long-context and broad exploration | Needs bounded, repo-aware context selection |
| OpenCode | Open agent shell | Flexible local/remote model routing | Still benefits from semantic repository memory |
| Qwen/Kimi/local models | Cheap execution/review | Low cost for routine tasks | Need curated context to stay accurate |
| CodeAware v4 | Persistent semantic context runtime | Controls context, budgets, traces and semantic retrieval | Still needs agents/models to execute reasoning and patches |

Best setup:

```text
Cursor / Claude Code / Gemini CLI / OpenCode
        ↓
CodeAware MCP
        ↓
SemanticIndex + ContextPackage + Budget + Trace
        ↓
Repository
```

---

## 🧩 Typical use cases

### 1. Reduce Claude Code token burn

Instead of asking an agent to inspect the whole repository, ask CodeAware first:

```text
codeaware.get_task_context(goal="Fix login session handling")
```

Then feed the returned context package to the agent.

### 2. Find exactly where a symbol lives

```text
codeaware.find_symbol(query="ContextPackage")
```

This avoids loading unrelated files.

### 3. Find callers before editing

```text
codeaware.find_callers(symbol="build_context")
```

Useful before refactors, renames and behavior changes.

### 4. Find related tests

```text
codeaware.find_tests(symbol="ContextPackage")
```

Useful for minimal test selection.

### 5. Estimate change impact

```text
codeaware.diff_impact(changed_path="src/v4/tools.rs")
```

Useful before committing or asking a model to make a risky edit.

---

## 🧱 Design principles

### 1. Context is owned by the runtime

The model should not decide freely how much of the repository to read.

```text
Agent requests context.
CodeAware decides what context is allowed.
```

### 2. Semantic first, file summary second

CodeAware first tries semantic context:

```text
symbols → imports → calls → tests → impact
```

Only when no semantic context is available does it fall back to file summaries.

### 3. Bounded execution

Every task should have limits:

```text
max files read
max files changed
max tool calls
max context tokens
stop conditions
```

### 4. Trace everything

Every context package should be explainable:

```text
Why was this file selected?
Why was this path excluded?
How many estimated tokens were used?
```

### 5. Local-first by default

Repository intelligence should be available without sending the entire codebase to external services.

---

## 🧪 Status honesty

The v4 architecture, runtime modules, semantic APIs and MCP dispatcher wiring are implemented.

However, a repository is only production-ready after CI/build verification.

Current truth:

```text
Implemented: yes
Documented: yes
MCP-dispatch wired: yes
CI workflow added: yes
CI green: must be verified from GitHub Actions after workflow execution
```

Run locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
cargo build --release --all-features
```

---

## ⚡ Quick Start

### 1. Clone the repository

```bash
git clone https://github.com/mhmtbsbyndr/codeaware-mcp.git
cd codeaware-mcp
```

### 2. Build the MCP server

```bash
cargo build --release
```

The binary will be available at:

```bash
./target/release/codeaware-mcp
```

### 3. Run tests

```bash
cargo test
```

### 4. Run locally over stdio

```bash
./target/release/codeaware-mcp
```

The server speaks **JSON-RPC over stdio**, as expected by MCP clients.

### 5. Optional: Keep dashboard running in background (macOS / Linux)

```bash
chmod +x scripts/setup-codeaware-mcp-dashboard-launchd.sh
./scripts/setup-codeaware-mcp-dashboard-launchd.sh install /usr/local/bin/codeaware-mcp
```

Useful follow-ups:

- `./scripts/setup-codeaware-mcp-dashboard-launchd.sh stop`
- `./scripts/setup-codeaware-mcp-dashboard-launchd.sh start`
- `./scripts/setup-codeaware-mcp-dashboard-launchd.sh uninstall`

Supported OS:

- macOS: LaunchAgent (Launchd)
- Linux (user systemd): systemd user unit

Linux usage:

```bash
./scripts/setup-codeaware-mcp-dashboard-launchd.sh install /home/$USER/.cargo/bin/codeaware-mcp
```

To verify, call the MCP `xray` tool and open the returned URL (for example `http://127.0.0.1:9847`).

### 6. Add it to Claude Code / MCP config

Example `.mcp.json`:

```json
{
  "mcpServers": {
    "codeaware": {
      "command": "/absolute/path/to/codeaware-mcp/target/release/codeaware-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

Use an absolute path for the binary when configuring an MCP client.

---

## ✅ Requirements

| Requirement | Version / Notes |
|---|---|
| Rust | 2021 edition compatible toolchain |
| Cargo | Included with Rust |
| SQLite | Used by the existing session/memory foundation |
| Git | Required for git intelligence tools |
| Claude Code or MCP client | Any client supporting stdio MCP servers |

Recommended setup:

```bash
rustup update
cargo build --release
cargo test
```

---

## 🧭 Why v4 matters

AI coding agents often waste context on:

```text
Read("src/server.rs")       -> hundreds of source lines
Run("cargo test")           -> hundreds of noisy log lines
Read("src/server.rs") again -> same content again
Context compaction           -> working memory disappears
```

CodeAware v4 moves toward:

```text
codeaware.get_task_context   -> bounded semantic context package
codeaware.find_symbol        -> symbol-level retrieval
codeaware.find_callers       -> caller graph lookup
codeaware.find_tests         -> related tests
codeaware.diff_impact        -> impact-aware reasoning
semantic_router              -> cheap/balanced/premium model routing hint
semantic_recovery            -> compact task recovery snapshot
```

The goal is not just fewer tokens.

The goal is **better, denser, bounded semantic context**.

---

## 🏗️ Architecture

```text
AI Coding Agent
      |
      v
MCP JSON-RPC / stdio
      |
      v
codeaware-mcp Runtime
      |
      +-- Stable Compression Layer
      |     +-- smart_read
      |     +-- smart_run
      |     +-- git intelligence
      |
      +-- v4 Persistent Code Intelligence Kernel
      |     +-- task contracts
      |     +-- budget engine
      |     +-- discovery/ranking
      |     +-- summaries/token estimation
      |     +-- context packages
      |     +-- semantic index
      |     +-- symbols/imports/calls/tests
      |     +-- impact analysis
      |     +-- architecture memory
      |     +-- semantic recovery
      |     +-- semantic router
      |
      +-- Token Runtime
      |     +-- token_stats
      |     +-- token_savings_report
      |     +-- benchmark_compression
      |
      +-- Context Optimization Runtime
      |     +-- get_relevant_code
      |     +-- code_search
      |     +-- get_relevant_test_errors
      |     +-- get_project_context
      |     +-- tool_manager
      |
      +-- Progressive Memory Foundation
      |     +-- compact memory index
      |     +-- timeline window
      |     +-- observation details
      |     +-- privacy tag filtering
      |     +-- memory citations
      |
      +-- Safety Foundation
            +-- security policy
            +-- command validation
            +-- path validation
            +-- MCP routing
```

---

## 🧪 Verify the server

Run tests:

```bash
cargo test
```

Start the binary manually:

```bash
./target/release/codeaware-mcp
```

Example JSON-RPC initialize call:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

Example existing tool call:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"token_stats","arguments":{}}}
```

Example v4 semantic tool call:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"codeaware.get_task_context","arguments":{"repo_root":".","goal":"Explain v4 semantic context"}}}
```

---

## 🗂️ v4 Documentation

The v4 architecture is documented in:

```text
docs/CODEAWARE_V4_MASTERPLAN.md
docs/CODEAWARE_V4_ROADMAP.md
docs/CODEAWARE_V4_PHASE1_IMPLEMENTATION_SPEC.md
docs/CODEAWARE_V4_IMPLEMENTATION_SNAPSHOT.md
docs/CODEAWARE_V4_PHASE2_STATUS.md
docs/CODEAWARE_V4_FINAL_ARCHITECTURE.md
docs/CODEAWARE_V4_MCP_TOOLS.md
```

---

## 🚦 Current status

CodeAware currently has:

- real Rust crate structure,
- real MCP stdio server,
- real JSON-RPC dispatch for existing tools,
- v4 semantic MCP tool dispatch,
- stable compression-oriented MCP tools,
- runtime-wired token/quality/benchmark/context tools,
- progressive memory foundations,
- v4 semantic code intelligence runtime foundations,
- semantic-first context assembly APIs,
- persistent semantic repository kernel design.

Remaining production-hardening tasks:

```text
- run full cargo test in CI/local environment
- fix any compile/test regressions if found
- extend tree-sitter support beyond Rust extraction
- add production-grade AST call extraction
- improve semantic index cache invalidation strategy
```

---

## 🛣️ Roadmap

### Near-term

- Confirm GitHub Actions CI is green.
- Tighten `tools/list` metadata for v4 tools.
- Improve semantic index persistence and cache invalidation.
- Add benchmarks for token savings versus raw file reads.

### Mid-term

- Add TypeScript/JavaScript/Python/PHP/Go/Swift/Java extraction.
- Replace heuristic call graph with AST-aware call extraction.
- Persist architecture memory and decisions with richer query support.
- Add semantic diffing across commits.

### Long-term

- Multi-model routing based on semantic complexity.
- Persistent cross-repository memory.
- Semantic task planner.
- Minimal test selection.
- IDE/LSP integration.

---

## 🧩 Development philosophy

Every feature should reduce at least one of these costs:

- repeated context reads,
- noisy terminal output,
- lost session memory,
- unsafe edits,
- unclear code impact,
- unverifiable AI claims,
- tool-schema overload,
- cross-repo blindness,
- quality loss from over-compression,
- uncontrolled semantic drift.

`codeaware-mcp` is not just about fewer tokens.

It is about **better tokens and persistent semantic code intelligence**.
