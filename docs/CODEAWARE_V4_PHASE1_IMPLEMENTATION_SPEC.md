# CodeAware v4 Phase 1 Implementation Spec

**Phase:** 1  
**Name:** Budget & Contract Kernel  
**Goal:** Prevent uncontrolled AI-agent repository exploration and token burn before deeper symbol-graph work begins.

---

## 1. Phase 1 Scope

Phase 1 must add a controlled v4 layer without breaking existing CodeAware MCP behavior.

It introduces:

- task contracts,
- context budgets,
- read-once tracking,
- file summary caching,
- context package assembly,
- trace logging,
- and three initial MCP tools.

This phase does **not** require a full symbol graph yet.

---

## 2. New module layout

Add these modules under `src/v4/`:

```text
src/v4/
  mod.rs
  contracts.rs
  budget.rs
  context.rs
  trace.rs
  cache.rs
  errors.rs
  tools.rs
```

If the existing project uses a different structure, keep v4 isolated and expose it from the current MCP server entrypoint.

---

## 3. Core data models

### 3.1 TaskContract

```rust
pub struct TaskContract {
    pub task_id: String,
    pub intent: TaskIntent,
    pub goal: String,
    pub scope: TaskScope,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub stop_conditions: Vec<StopCondition>,
}
```

### 3.2 TaskScope

```rust
pub struct TaskScope {
    pub max_files_read: usize,
    pub max_files_changed: usize,
    pub max_tool_calls: usize,
    pub max_context_tokens: usize,
    pub max_output_tokens: Option<usize>,
}
```

### 3.3 TaskIntent

```rust
pub enum TaskIntent {
    Analyze,
    ImplementFeature,
    FixBug,
    Refactor,
    WriteTests,
    UpdateDocs,
    Review,
    Unknown,
}
```

### 3.4 StopCondition

```rust
pub enum StopCondition {
    ShowDiff,
    WaitForHuman,
    BudgetExceeded,
    ContractViolation,
    TestsRequired,
}
```

### 3.5 BudgetState

```rust
pub struct BudgetState {
    pub task_id: String,
    pub files_read: usize,
    pub files_changed: usize,
    pub tool_calls: usize,
    pub estimated_context_tokens: usize,
    pub max_files_read: usize,
    pub max_files_changed: usize,
    pub max_tool_calls: usize,
    pub max_context_tokens: usize,
}
```

### 3.6 ContextPackage

```rust
pub struct ContextPackage {
    pub task_id: String,
    pub repo_root: String,
    pub contract: TaskContract,
    pub budget: BudgetState,
    pub selected_context: Vec<ContextItem>,
    pub excluded_context: Vec<ExcludedContext>,
    pub warnings: Vec<String>,
}
```

### 3.7 ContextItem

```rust
pub struct ContextItem {
    pub kind: ContextItemKind,
    pub path: Option<String>,
    pub symbol: Option<String>,
    pub content: String,
    pub reason: String,
    pub estimated_tokens: usize,
}
```

### 3.8 ContextItemKind

```rust
pub enum ContextItemKind {
    FileSummary,
    FileExcerpt,
    ArchitectureRule,
    TestHint,
    RecentChange,
    DecisionRecord,
    Contract,
}
```

---

## 4. Storage layout

Use a local `.codeaware/` directory in the repository root.

```text
.codeaware/
  v4/
    cache/
      file_summaries.jsonl
      read_once.jsonl
    traces/
      task_traces.jsonl
    contracts/
      active_contracts.jsonl
    decisions/
      decisions.jsonl
```

All files should be append-friendly and resilient to corruption.

If a file is malformed, CodeAware should warn and continue with an empty state rather than crashing the MCP server.

---

## 5. Budget behavior

### 5.1 Default budget

If no budget is provided, use conservative defaults:

```text
max_files_read: 8
max_files_changed: 4
max_tool_calls: 12
max_context_tokens: 30000
```

### 5.2 Budget checks

`check_budget` must return:

```json
{
  "ok": true,
  "remaining": {
    "files_read": 3,
    "tool_calls": 8,
    "context_tokens": 12000
  },
  "warnings": []
}
```

If exceeded:

```json
{
  "ok": false,
  "reason": "max_files_read exceeded",
  "stop_condition": "BudgetExceeded"
}
```

---

## 6. Context assembly behavior

`get_task_context` must:

1. accept a goal and optional contract,
2. create or load a task contract,
3. identify candidate files using existing CodeAware capabilities where available,
4. prefer cached summaries over full file content,
5. include only highly relevant excerpts/summaries,
6. exclude generated directories,
7. return a single compact context package,
8. write a trace entry explaining selected and excluded context.

### 6.1 Default excluded paths

```text
.git/**
target/**
node_modules/**
vendor/**
dist/**
build/**
.cache/**
.codeaware/**
```

### 6.2 Context ranking rules

Prefer in this order:

1. explicitly mentioned files,
2. files matching goal keywords,
3. existing summaries,
4. nearby tests,
5. docs/contracts relevant to the goal,
6. recent change memory,
7. raw excerpts only when summaries are insufficient.

---

## 7. MCP tools for Phase 1

### 7.1 `codeaware.create_task_contract`

Input:

```json
{
  "goal": "Implement read-once cache",
  "intent": "ImplementFeature",
  "allowed_paths": ["src/v4/**"],
  "max_files_read": 8,
  "max_files_changed": 4,
  "max_tool_calls": 12,
  "max_context_tokens": 30000
}
```

Output:

```json
{
  "task_id": "uuid",
  "contract": { }
}
```

### 7.2 `codeaware.check_budget`

Input:

```json
{
  "task_id": "uuid"
}
```

Output:

```json
{
  "ok": true,
  "budget": { },
  "warnings": []
}
```

### 7.3 `codeaware.get_task_context`

Input:

```json
{
  "goal": "Implement read-once cache",
  "intent": "ImplementFeature",
  "task_id": "optional-existing-task-id",
  "max_context_tokens": 30000
}
```

Output:

```json
{
  "task_id": "uuid",
  "context_package": { },
  "agent_instructions": [
    "Do not scan the full repository.",
    "Do not exceed the contract.",
    "Stop after showing the diff."
  ]
}
```

---

## 8. Agent instructions emitted by v4

Every `get_task_context` response must include instructions suitable for Claude Code, Gemini CLI, Cursor or OpenCode:

```text
You are operating under a CodeAware v4 task contract.
Do not scan the full repository.
Do not open files outside allowed paths unless a new contract is created.
Do not re-read files already summarized in this context package.
Stop after one implementation step and show the diff.
```

---

## 9. Trace logging

Each context assembly must append a trace entry:

```json
{
  "task_id": "uuid",
  "timestamp": "ISO-8601",
  "goal": "Implement read-once cache",
  "selected": [
    { "path": "src/v4/cache.rs", "reason": "target module" }
  ],
  "excluded": [
    { "path": "target/**", "reason": "generated" }
  ],
  "budget": {
    "estimated_context_tokens": 18400,
    "max_context_tokens": 30000
  }
}
```

---

## 10. Acceptance criteria

Phase 1 is done when:

- v4 modules compile,
- task contracts can be created,
- budget state can be checked,
- context packages can be generated,
- traces are persisted,
- generated directories are excluded by default,
- output includes agent instructions,
- existing MCP functionality remains intact,
- tests cover contract creation, budget overflow and context package assembly.

---

## 11. Test plan

Add tests for:

1. default contract creation,
2. custom contract creation,
3. budget remaining calculation,
4. budget overflow detection,
5. forbidden path exclusion,
6. context package contains selected and excluded entries,
7. trace append does not crash on malformed existing trace file.

---

## 12. First implementation order

Implement in this order:

```text
1. src/v4/mod.rs
2. src/v4/errors.rs
3. src/v4/contracts.rs
4. src/v4/budget.rs
5. src/v4/cache.rs
6. src/v4/trace.rs
7. src/v4/context.rs
8. src/v4/tools.rs
9. MCP server registration
10. tests
```

---

## 13. Non-goals

Do not implement in Phase 1:

- full semantic embeddings,
- full symbol graph,
- model router,
- external vector DB,
- web dashboard,
- multi-agent orchestration.

Those belong to later phases.
