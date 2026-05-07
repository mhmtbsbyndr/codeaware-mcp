# CodeAware v4 Implementation Snapshot

**Status:** Initial Phase 1 runtime implemented  
**Target:** Persistent Code Intelligence Kernel for bounded AI coding context

---

## Implemented runtime modules

```text
src/v4/
  mod.rs
  contracts.rs
  budget.rs
  cache.rs
  context.rs
  context_items.rs
  errors.rs
  ranking.rs
  storage.rs
  summaries.rs
  tokens.rs
  tools.rs
  trace.rs
```

---

## Implemented capabilities

### Contract layer

- `TaskContract`
- `TaskScope`
- `TaskIntent`
- `StopCondition`
- safe default contract generation
- default read/edit/tool/context limits

### Budget layer

- `BudgetState`
- `BudgetRemaining`
- `BudgetCheck`
- budget overflow detection
- near-context-exhaustion warning

### Context package layer

- `ContextPackage`
- `ContextItem`
- `ContextItemKind`
- `ExcludedContext`
- contract-first context package generation
- forbidden context output
- agent instruction generation

### Cache layer

- `FileSummary`
- `ReadOnceCache`
- basic read tracking

### Trace layer

- `TraceEntry`
- task id
- timestamp
- goal
- selected/excluded paths
- estimated context tokens

### Storage layer

- `.codeaware/v4/` layout support
- cache/traces/contracts/decisions directories
- JSONL append helper
- resilient directory creation

### Ranking layer

- `RankedPath`
- `ContextRanker`
- keyword-based path relevance ranking

### Token layer

- rough token estimation
- path relevance estimation

### Summary layer

- `GeneratedSummary`
- `SummaryGenerator`
- summary-first context generation
- estimated token count per generated summary

### Tool layer

- `CreateTaskContractRequest`
- `CheckBudgetRequest`
- `GetTaskContextRequest`
- `GetTaskContextResponse`
- `V4Tools::default_contract`
- `V4Tools::get_task_context`
- `V4Tools::agent_instructions`

---

## Implemented tests

```text
tests/v4_budget_tests.rs
tests/v4_context_tests.rs
tests/v4_ranking_tests.rs
```

Covered so far:

- budget passes within limits
- default contracts use safe limits
- `get_task_context` returns task id, contract and agent instructions
- excluded paths exist in context package
- token estimation returns non-zero values
- ranking prefers matching paths

---

## Current behavior

The v4 kernel can now produce a first conservative context package:

```text
user goal
→ task contract
→ summary-first context item
→ estimated token budget
→ excluded context list
→ agent instructions
```

This is intentionally conservative. The current Phase 1 runtime does not yet perform full repo retrieval, symbol extraction or MCP registration.

---

## Architecture shift already achieved

Before v4:

```text
Agent → Repo scan → Large context → Token burn
```

With v4 foundation:

```text
Agent → CodeAware v4 → TaskContract → ContextPackage → Bounded execution
```

---

## Next required block

### Block A — MCP Server Registration

Expose these as real MCP tools:

- `codeaware.create_task_contract`
- `codeaware.check_budget`
- `codeaware.get_task_context`

### Block B — File Candidate Discovery

Add candidate file discovery using existing CodeAware internals or a simple initial walker:

- ignore generated directories
- collect candidate paths
- rank by goal relevance
- summarize top candidates
- respect max files read

### Block C — Trace Persistence

Persist every `get_task_context` call to:

```text
.codeaware/v4/traces/task_traces.jsonl
```

### Block D — Read-once Enforcement

Use `ReadOnceCache` to warn or block repeated reads inside the same task.

### Block E — Symbol Intelligence

Later phase:

- tree-sitter symbols
- imports
- callers
- tests
- impact graph

---

## Design principle to keep

The LLM must not freely own repository context.

CodeAware v4 owns context selection, budget, constraints and traceability.
