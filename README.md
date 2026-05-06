# 🧠 codeaware-mcp

<p align="center">
  <strong>Local-first AI Code Intelligence, Token Compression, Progressive Memory, Research & Quality Runtime for MCP Agents</strong>
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

> **Stable core compression + runtime-wired v3 foundation for token quality, feedback, context optimization, progressive memory, benchmarks and future code intelligence.**

This README intentionally separates what is **stable**, what is **runtime-wired foundation**, and what is still **planned/provider-backed**.

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

### 3. Run locally over stdio

```bash
./target/release/codeaware-mcp
```

The server speaks **JSON-RPC over stdio**, as expected by MCP clients.

### 4. Add it to Claude Code / MCP config

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

### 5. Restart Claude Code

After restarting, the `codeaware` MCP server should appear as an available tool provider.

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

Example tool call:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"token_stats","arguments":{}}}
```

Example context optimizer call:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_relevant_code","arguments":{"query":"login","source":"fn login() {}\nfn logout() {}","max_snippets":5}}}
```

---

## ⚙️ Configuration

`codeaware-mcp` is designed to work with a local configuration file such as:

```text
.codeaware.toml
```

Example:

```toml
[context]
compression_level = "medium"
tool_policy = "focus_tools"
extended_thinking_policy = "auto"

[security]
redact_secrets = true
allow_network_access = false

[memory]
enable_progressive_retrieval = true
respect_private_tags = true
```

The current stable runtime also supports project/session persistence through the existing internal session database.

---

## 🧰 Common usage patterns

### Explore a project

Use:

```text
project_map
smart_read
code_search
get_relevant_code
```

### Debug failing tests

Use:

```text
smart_run
get_relevant_test_errors
token_quality
```

### Reduce context overhead

Use:

```text
get_project_context
tool_manager
token_stats
token_savings_report
```

### Work with memory

Use:

```text
save_memory
search_memory
memory_timeline
summarize_memory
```

Progressive memory foundations add:

```text
compact index -> timeline -> full details
privacy tags
memory citations
```

---

## ✅ Current capability status

| Feature category | Included? | Status | Notes |
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
| Context optimizer | ✅ | Runtime-wired foundation | Relevant snippets, test-error extraction, project-context reduction, tool policies |
| Progressive memory retrieval | ✅ | Foundation | Compact index → timeline → detail workflow, privacy tags, citations |
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
smart_read                  -> symbols, imports, focused context
token_stats                 -> measurable token savings
token_quality               -> quality rating from test/build signals
benchmark_compression       -> reproducible compression metrics
get_relevant_code           -> only matching code snippets
get_relevant_test_errors    -> only failure signals and impacted files
get_project_context         -> compact project instructions
progressive_memory_plan     -> compact index -> timeline -> full detail pattern
provide_feedback            -> human-in-the-loop improvement signal
```

The goal is not to increase a model's raw context window. The goal is to increase **effective usable context** through relevance density, progressive retrieval and quality-per-token.

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
      +-- Context Optimization Runtime
      |     +-- get_relevant_code
      |     +-- code_search
      |     +-- get_relevant_test_errors
      |     +-- get_project_context
      |     +-- tool_manager
      |
      +-- Quality Runtime
      |     +-- token_quality
      |     +-- provide_feedback
      |     +-- compression experiments
      |
      +-- Progressive Memory Foundation
      |     +-- compact memory index
      |     +-- timeline window
      |     +-- observation details
      |     +-- privacy tag filtering
      |     +-- memory citations
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

## 🧰 Full MCP tool inventory

### Stable / existing tools

| Tool | Purpose |
|---|---|
| `project_map` | Generate a compressed structural overview of a project |
| `smart_read` | Read code with skeleton/focused/diff/full modes |
| `smart_edit` | Edit files with impact/conflict-aware strategies |
| `smart_run` | Run commands with compressed output capture |
| `session_status` | Report current session state and compaction context |
| `workspace_state` | Read/write typed workspace slots |
| `validate_config` | Validate CodeAware configuration |
| `xray` | Open live metrics/dashboard view |
| `save_memory` | Save persistent semantic observations |
| `search_memory` | Search persistent memory |
| `memory_timeline` | Retrieve observations around an anchor |
| `summarize_memory` | Cluster and deduplicate observations |
| `git_diff` | Structured git diff summaries |
| `git_blame` | Structured blame context |
| `git_changelog` | Conventional changelog generation |
| `smart_refactor` | AST-aware rename/refactor preview |
| `test_coverage_map` | Function-level heuristic test coverage map |

### Runtime-wired v3 foundation tools

| Tool | Purpose | Status |
|---|---|---|
| `token_stats` | Returns a token accounting summary | Runtime-wired foundation |
| `token_savings_report` | Returns a Markdown savings report | Runtime-wired foundation |
| `benchmark_compression` | Runs deterministic compression benchmark logic | Runtime-wired foundation |
| `provide_feedback` | Accepts human feedback ratings and comments | Runtime-wired foundation |
| `token_quality` | Evaluates quality from test/build signals | Runtime-wired foundation |
| `get_relevant_code` | Returns relevant code snippets instead of whole files | Runtime-wired foundation |
| `code_search` | Finds relevant snippets with file/symbol hints | Runtime-wired foundation |
| `get_relevant_test_errors` | Extracts test/build errors and impacted files | Runtime-wired foundation |
| `get_project_context` | Reduces long instructions/docs to architecture and rules | Runtime-wired foundation |
| `tool_manager` | Selects enabled tools and thinking policy by context policy | Runtime-wired foundation |

---

## 🔎 Context optimizer tools

These tools implement the **effective context window expansion** strategy: fewer irrelevant tokens, more useful signal.

### `get_relevant_code`

Returns only matching code snippets rather than whole files.

```json
{
  "name": "get_relevant_code",
  "arguments": {
    "query": "login",
    "source": "fn login() {}\nfn logout() {}",
    "max_snippets": 5
  }
}
```

### `code_search`

Searches source content and returns file/symbol/line/snippet matches.

```json
{
  "name": "code_search",
  "arguments": {
    "query": "AuthManager",
    "path": "src/auth.rs",
    "source": "struct AuthManager {}",
    "max_results": 8
  }
}
```

### `get_relevant_test_errors`

Extracts relevant failure lines and impacted files from noisy test/build output.

```json
{
  "name": "get_relevant_test_errors",
  "arguments": {
    "output": "test failed\nerror in src/auth.rs:10",
    "duration_ms": 1200
  }
}
```

### `get_project_context`

Compresses long project instructions or docs into architecture and key rules.

```json
{
  "name": "get_project_context",
  "arguments": {
    "source": "Architecture: Rust MCP runtime\nYou must keep responses deterministic\nLong example text"
  }
}
```

### `tool_manager`

Selects enabled tools and extended-thinking behavior based on a context policy.

```json
{
  "name": "tool_manager",
  "arguments": {
    "context_compression_level": "aggressive",
    "tool_policy": "minimal_tools",
    "extended_thinking_policy": "auto",
    "tools_to_enable": ["smart_read", "web_search", "token_stats"]
  }
}
```

---

## 🧠 Progressive Memory Retrieval

Inspired by layered memory retrieval patterns, `codeaware-mcp` includes a progressive memory foundation:

```text
Layer 1: compact index search
Layer 2: timeline/context window around selected IDs
Layer 3: full observation details only for final filtered IDs
```

This avoids loading full memories too early and improves context density.

### Privacy tags

`codeaware-mcp` recognizes private memory sections:

```text
<private>
Do not inject this into future context.
</private>
```

### Memory citations

Memory observations can be cited with stable IDs:

```text
memory://observation/42
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
| `src/context_optimizer.rs` | Effective context-window optimization runtime | Runtime-wired foundation |
| `src/progressive_memory.rs` | Progressive disclosure memory retrieval, privacy tags and citations | Foundation |
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

`codeaware-mcp` includes a quality model so compression can be judged by outcome, not just smaller output.

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
- privacy-tag filtering,
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
- context optimizer tools
- progressive memory foundations
- compression experiments
- MCP dispatch wiring

### Phase 2 — Persistence & real metrics

- SQLite-backed token events
- persisted feedback
- persisted quality history
- persisted progressive memory index
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
- runtime-wired token/quality/benchmark/context tools,
- progressive memory foundations,
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
