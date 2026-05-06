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

It is evolving from a token-saving helper into a full:

> **AI Code Intelligence / Compression / Research / Quality Optimization Runtime**

---

## ✨ Highlights

| Capability | Status | Description |
|---|---:|---|
| 🧮 Token Accounting | ✅ Runtime wired | Measure raw vs compressed token usage |
| 📊 Savings Reports | ✅ Runtime wired | Generate Markdown token savings reports |
| 🧪 Compression Benchmarks | ✅ Runtime wired | Validate compression fixtures and savings ratios |
| ⭐ Token Quality Monitor | ✅ Foundation | Good/Warning/Bad evaluation from test signals |
| 💬 Human Feedback Layer | ✅ Foundation | Feedback entries, ratings, aggregation |
| 🧪 A/B Compression Experiments | ✅ Foundation | Compare compression pipelines by quality and token cost |
| 🧠 Symbol Index | 🧱 Foundation | Repository graph: files, symbols, references, call edges |
| 🔎 Deep Research | 🧱 Foundation | Evidence-based answers from code structure |
| 🧩 Multi-Repo Workspace | 🧱 Foundation | Workspace maps and cross-repo awareness |
| 🧬 LSP Bridge | 🧱 Foundation | Hover, definitions, references, diagnostics abstraction |
| 🌐 Browser Awareness | 🧱 Foundation | DOM, console, and network summaries |
| 🛡️ Security Policy | 🧱 Foundation | Command/path validation and policy findings |
| 🔀 MCP Router | 🧱 Foundation | Capability-based MCP routing skeleton |

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
      +-- Compression Layer
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
      +-- Code Intelligence Layer
      |     +-- symbol index
      |     +-- references
      |     +-- callers / callees
      |     +-- outline extraction
      |
      +-- Research Layer
      |     +-- deep_research
      |     +-- evidence model
      |     +-- suggested next actions
      |
      +-- Workspace Layer
      |     +-- workspace map
      |     +-- cross-repo search
      |
      +-- Integration Layer
      |     +-- LSP bridge
      |     +-- browser/CDP awareness
      |
      +-- Safety Layer
            +-- security policy
            +-- command validation
            +-- path validation
            +-- MCP routing
```

---

## ✅ Runtime-wired MCP tools

These tools are connected to the real MCP `tools/call` dispatch path.

| Tool | Purpose |
|---|---|
| `token_stats` | Returns a token accounting summary |
| `token_savings_report` | Returns a Markdown savings report |
| `benchmark_compression` | Runs deterministic compression benchmark logic |
| `provide_feedback` | Accepts human feedback ratings and comments |
| `token_quality` | Evaluates quality from test/build signals |

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

## 📦 Runtime modules

| Module | Purpose |
|---|---|
| `src/token_stats.rs` | Token events, deterministic estimation, aggregation |
| `src/token_stats_persistence.rs` | Token event persistence contract and SQL schema |
| `src/token_stats_tools.rs` | Tool DTOs and Markdown report rendering |
| `src/token_benchmark.rs` | Compression benchmark runtime |
| `src/token_quality.rs` | Quality rating from test/build signals |
| `src/feedback_layer.rs` | Human-in-the-loop feedback model |
| `src/experiment_layer.rs` | Compression pipeline experiments |
| `src/symbol_index.rs` | Repository symbol graph foundation |
| `src/deep_research.rs` | Evidence-based research skeleton |
| `src/workspace_awareness.rs` | Multi-repo workspace model |
| `src/lsp_bridge.rs` | Editor/LSP bridge abstraction |
| `src/browser_awareness.rs` | Browser/CDP summary abstraction |
| `src/security_policy.rs` | Security policy and validation findings |
| `src/mcp_router.rs` | MCP route and capability router |
| `src/tools/foundation.rs` | Live MCP handlers for foundation tools |

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

---

## 🧠 Token Quality Monitoring

`codeaware-mcp` now includes a quality model so compression can be judged by outcome, not just smaller output.

| Rating | Meaning |
|---|---|
| `Good` | Tests/build signals are clean |
| `Warning` | Tests pass, but signals are suspicious, slow, or degraded |
| `Bad` | Tests/build failed |

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

### Phase 1 — Token & Quality Runtime ✅

- token accounting
- savings reports
- benchmark runtime
- token quality monitoring
- feedback layer
- compression experiments
- MCP dispatch wiring

### Phase 2 — Persistence & Real Metrics

- SQLite-backed token events
- persisted feedback
- persisted quality history
- benchmark fixture loader
- dashboard panels

### Phase 3 — Code Intelligence Runtime

- persistent symbol index
- tree-sitter ingestion
- references/callers/callees tools
- impact analysis
- hotspot ranking

### Phase 4 — Research & Workspace Intelligence

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

`codeaware-mcp` now has:

- real Rust crate structure,
- real MCP stdio server,
- real JSON-RPC dispatch,
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
