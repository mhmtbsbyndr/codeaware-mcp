# 🧠 codeaware-mcp

<p align="center">
  <strong>Local-first AI Code Intelligence, Token Compression, Research & Quality Runtime for MCP Agents</strong>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?style=for-the-badge&logo=rust" />
  <img alt="MCP" src="https://img.shields.io/badge/MCP-JSON--RPC-blue?style=for-the-badge" />
  <img alt="Local First" src="https://img.shields.io/badge/Local--First-Yes-success?style=for-the-badge" />
  <img alt="Status" src="https://img.shields.io/badge/Status-v3%20Foundation-purple?style=for-the-badge" />
</p>

---

## 🚀 What is codeaware-mcp?

`codeaware-mcp` is a **local-first MCP runtime** for AI coding agents such as Claude Code, Codex-style agents, and local LLM workflows.

It sits between the agent and your repository and returns **compressed, structured, evidence-based code intelligence** instead of raw files, noisy terminal output, repeated diffs, and unmeasured token usage.

The current project is best described as:

> **Stable core compression + runtime-wired v3 foundation for token quality, feedback, benchmarks and future code intelligence.**

This README intentionally separates what is **stable**, what is **runtime-wired foundation**, and what is still **planned**.

---

## ✅ Current capability status

| Feature category | Currently included? | Status | Notes |
|---|---:|---|---|
| Tree-sitter based AST/code compression | ✅ | Stable core | Core code-aware compression concept |
| Terminal / run output compression | ✅ | Stable core | Reduces build/test output to relevant signals |
| Git intelligence and session reconstruction | ✅ | Stable core | Git diff, changelog, blame and session context support |
| Persistent session/memory layer | ✅ | Stable core | Existing SQLite/session foundation |
| XRay dashboard | ✅ | Stable core | Existing live metrics/dashboard layer |
| Token accounting runtime | ✅ | Runtime-wired foundation | `token_stats` tool wired into MCP dispatch |
| Token savings reports | ✅ | Runtime-wired foundation | `token_savings_report` tool wired into MCP dispatch |
| Compression benchmark runtime | ✅ | Runtime-wired foundation | `benchmark_compression` tool wired into MCP dispatch |
| Token quality monitor | ✅ | Runtime-wired foundation | Quality model exists; DB-backed history is next |
| Human feedback layer | ✅ | Runtime-wired foundation | `provide_feedback` validates feedback; persistence is next |
| A/B compression experiments | ✅ | Foundation | Pipeline model and report logic exist |
| Symbol index / code graph | 🧱 | Foundation | Models and query logic exist; provider ingestion is next |
| Deep research layer | 🧱 | Foundation | Evidence model exists; full provider wiring is next |
| LSP bridge | 🧱 | Foundation | Abstraction exists; process/provider wiring is next |
| Browser/CDP awareness | 🧱 | Foundation | Summary model exists; runtime browser connector is next |
| Security / policy layer | 🧱 | Foundation | Validation model exists; wider enforcement is next |
| MCP router / orchestration | 🧱 | Foundation | Capability routing skeleton exists |

Legend:

- ✅ **Stable core**: implemented as part of the existing MCP server capability set.
- ✅ **Runtime-wired foundation**: handler is connected to the MCP `tools/call` dispatch path, but deeper persistence/provider wiring may still evolve.
- 🧱 **Foundation**: models and core logic exist, but external providers or full runtime integration are roadmap work.

---

## 🧭 Why it matters

AI coding agents often waste context on:

```text
Read("src/server.rs")       -> hundreds of source lines
Run("cargo test")           -> hundreds of noisy log lines
Read("src/server.rs") again -> same content again
Context compaction           -> working memory disappears
```

`codeaware-mcp` aims to turn that into:

```text
smart_read              -> symbols, imports, focused context
token_stats             -> measurable token savings
token_quality           -> quality rating from test/build signals
benchmark_compression   -> reproducible compression metrics
deep_research           -> evidence-based repository answers
query_symbols           -> graph-aware code navigation
provide_feedback        -> human-in-the-loop improvement signal
```

The goal is not only to save tokens. The goal is to make AI coding agents cheaper, safer, more evidence-based, and measurable over time.

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
      +-- Token Runtime
      |     +-- token_stats
      |     +-- token_savings_report
      |     +-- benchmark_compression
      |
      +-- Quality Runtime
      |     +-- token_quality
      |     +-- provide_feedback
      |     +-- compression experiments
      |
      +-- Code Intelligence Foundation
      |     +-- symbol index
      |     +-- references
      |     +-- callers / callees
      |     +-- outline extraction
      |
      +-- Research Foundation
      |     +-- deep_research
      |     +-- evidence model
      |     +-- suggested next actions
      |
      +-- Workspace Foundation
      |     +-- workspace map
      |     +-- cross-repo search
      |
      +-- Integration Foundations
      |     +-- LSP bridge
      |     +-- browser/CDP awareness
      |
      +-- Safety Foundation
            +-- security policy
            +-- command validation
            +-- path validation
            +-- MCP routing
```

---

## ✅ Runtime-wired MCP tools added in v3 foundation

These tools are connected to the real MCP `tools/call` dispatch path.

| Tool | Purpose | Status |
|---|---|---|
| `token_stats` | Returns a token accounting summary | Runtime-wired foundation |
| `token_savings_report` | Returns a Markdown savings report | Runtime-wired foundation |
| `benchmark_compression` | Runs deterministic compression benchmark logic | Runtime-wired foundation |
| `provide_feedback` | Accepts human feedback ratings and comments | Runtime-wired foundation |
| `token_quality` | Evaluates quality from test/build signals | Runtime-wired foundation |

Example:

```json
{
  "name": "token_quality",
  "arguments": {
    "command": "cargo test",
    "exit_code": 0,
    "passed": 42,
    "failed": 0,
    "duration_ms": 1500
  }
}
```

---

## 📦 v3 foundation modules

| Module | Purpose | Status |
|---|---|---|
| `src/token_stats.rs` | Token events, deterministic estimation, aggregation | Runtime-wired foundation |
| `src/token_stats_persistence.rs` | Token event persistence contract and SQL schema | Foundation |
| `src/token_stats_tools.rs` | Tool DTOs and Markdown report rendering | Runtime-wired foundation |
| `src/token_benchmark.rs` | Compression benchmark runtime | Runtime-wired foundation |
| `src/token_quality.rs` | Quality rating from test/build signals | Runtime-wired foundation |
| `src/feedback_layer.rs` | Human-in-the-loop feedback model | Runtime-wired foundation |
| `src/experiment_layer.rs` | Compression pipeline experiments | Foundation |
| `src/symbol_index.rs` | Repository symbol graph foundation | Foundation |
| `src/deep_research.rs` | Evidence-based research skeleton | Foundation |
| `src/workspace_awareness.rs` | Multi-repo workspace model | Foundation |
| `src/lsp_bridge.rs` | Editor/LSP bridge abstraction | Foundation |
| `src/browser_awareness.rs` | Browser/CDP summary abstraction | Foundation |
| `src/security_policy.rs` | Security policy and validation findings | Foundation |
| `src/mcp_router.rs` | MCP route and capability router | Foundation |
| `src/tools/foundation.rs` | Live MCP handlers for foundation tools | Runtime-wired foundation |

---

## 🧪 Benchmark fixtures

Fixtures live in:

```text
benches/fixtures/token_stats/
```

Naming convention:

```text
<category>_<tool>_<name>.raw.txt
<category>_<tool>_<name>.compressed.txt
<category>_<tool>_<name>.meta.json
```

Example metadata:

```json
{
  "category": "file_read",
  "tool": "smart_read",
  "language": "rust",
  "subject": "src/server.rs",
  "expected_min_savings_ratio": 0.50,
  "expected_max_savings_ratio": 0.98
}
```

---

## 🧠 Token Quality Monitoring

`codeaware-mcp` now includes a quality model so compression can be judged by outcome, not just smaller output.

| Rating | Meaning |
|---|---|
| `Good` | Tests/build signals are clean |
| `Warning` | Tests pass, but signals are suspicious, slow, or degraded |
| `Bad` | Tests/build failed |

This enables future reports such as:

```text
Pipeline: ast_diff_only
  - Average tokens: 1,200
  - Quality GOOD: 85%
  - Tests pass: 92%

Pipeline: git_only
  - Average tokens: 900
  - Quality GOOD: 70%
  - Tests pass: 65%
```

---

## 💬 Feedback Layer

Human feedback can be captured through `provide_feedback`.

| Rating | Meaning |
|---:|---|
| `1` | Good / correct |
| `2` | Partially correct |
| `3` | Incorrect |

This creates the foundation for adaptive compression policies.

---

## 🛡️ Security posture

Security is part of the runtime model:

- command deny-list checks,
- path deny-list checks,
- future secret redaction hooks,
- explicit security findings,
- local-first default behavior,
- future network/offline policy controls.

---

## 🗺️ Roadmap

### Phase 1 — Stable core + v3 runtime foundation ✅

- AST/code compression
- terminal output compression
- git intelligence
- token accounting
- savings reports
- benchmark runtime
- token quality monitoring
- feedback layer
- compression experiments
- MCP dispatch wiring

### Phase 2 — Persistence & real metrics

- SQLite-backed token events
- persisted feedback
- persisted quality history
- benchmark fixture loader
- dashboard panels

### Phase 3 — Code intelligence runtime

- persistent symbol index
- tree-sitter ingestion
- references/callers/callees tools
- impact analysis
- hotspot ranking

### Phase 4 — Research & workspace intelligence

- deep research with evidence
- workspace map tool
- cross-repo search
- multi-repo impact analysis

### Phase 5 — Integrations

- LSP provider bridge
- browser/CDP summary runtime
- CI failure summaries
- PR review plans
- minimal test selection

---

## 🚦 Current status

`codeaware-mcp` currently has:

- real Rust crate structure,
- real MCP stdio server,
- real JSON-RPC dispatch,
- stable compression-oriented MCP tools,
- real foundation tool handlers,
- runtime-wired token/quality/benchmark tools,
- architecture foundations for code intelligence and research.

Some modules are intentionally foundation-level and still need provider wiring, especially SQLite adapters, tree-sitter ingestion, LSP process integration, browser/CDP runtime connection, and dashboard UI updates.

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
- quality loss from over-compression.

`codeaware-mcp` is not just about fewer tokens.

It is about **better tokens**.
