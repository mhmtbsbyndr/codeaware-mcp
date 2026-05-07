# 🧠 codeaware-mcp

<p align="center">
  <strong>Local-first AI Code Intelligence, Token Compression, Progressive Memory, Semantic Repository Runtime & Quality Layer for MCP Agents</strong>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?style=for-the-badge&logo=rust" />
  <img alt="MCP" src="https://img.shields.io/badge/MCP-JSON--RPC-blue?style=for-the-badge" />
  <img alt="Local First" src="https://img.shields.io/badge/Local--First-Yes-success?style=for-the-badge" />
  <img alt="Status" src="https://img.shields.io/badge/Status-v4%20Kernel-purple?style=for-the-badge" />
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
  call_graph.rs
  context.rs
  context_items.rs
  contracts.rs
  discovery.rs
  errors.rs
  impact.rs
  import_graph.rs
  index_builder.rs
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
| Semantic tools: find_symbol/find_callers/find_tests/diff_impact | Implemented as runtime APIs |
| Architecture memory | Implemented foundation |
| Decision memory | Implemented foundation |
| Semantic recovery | Implemented foundation |
| Semantic router | Implemented foundation |

Important: some v4 tools exist as runtime APIs and still need full JSON-RPC MCP dispatch registration for production MCP clients.

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

### 5. Add it to Claude Code / MCP config

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
get_task_context             -> bounded semantic context package
find_symbol                  -> symbol-level retrieval
find_callers                 -> caller graph lookup
find_tests                   -> related tests
_diff_impact                 -> impact-aware reasoning
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
```

---

## 🚦 Current status

CodeAware currently has:

- real Rust crate structure,
- real MCP stdio server,
- real JSON-RPC dispatch for existing tools,
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
- wire v4 semantic APIs into MCP tools/call dispatch
- add persistent semantic cache invalidation
- extend tree-sitter support beyond Rust
- add production-grade AST call extraction
```

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
