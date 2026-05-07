# CodeAware v4 Masterplan

**Status:** v4 architecture draft / implementation baseline  
**Mission:** Reduce AI coding token waste by compiling repositories into persistent, queryable, graph-based code intelligence.  
**Core principle:** The LLM does not own the repository context. CodeAware does.

---

## 1. Why v4 exists

Current AI coding agents still behave too much like brute-force repo readers:

- they scan too many files,
- repeatedly read the same areas,
- re-hydrate context after compaction,
- lose architectural intent,
- and spend expensive model tokens on raw repository text.

CodeAware v4 changes the control point.

Instead of:

```text
User task -> Agent -> Repo scan -> Huge context -> Patch
```

v4 moves to:

```text
Repo -> Persistent Code Intelligence Kernel -> Task Contract -> Minimal Context -> Agent Patch
```

The agent is no longer allowed to freely consume the repository. It receives a bounded, pre-assembled, task-specific context package.

---

## 2. Positioning

Claude Code, Cursor, Gemini CLI, OpenCode and similar systems are coding agents.

**CodeAware v4 is the context kernel that controls, feeds and limits those agents.**

v4 is not just an MCP search helper. It is a persistent code intelligence layer with:

- repository compilation,
- symbol graph,
- dependency graph,
- architecture memory,
- task contracts,
- context budget control,
- trace memory,
- impact analysis,
- and model-routing readiness.

---

## 3. Nemesis ideas adopted

CodeAware v4 borrows the useful execution ideas from Nemesis, without becoming over-engineered.

| Nemesis concept | CodeAware v4 usage |
|---|---|
| Kernel-first execution | CodeAware is the source of context truth, not the LLM chat |
| Execution Contract Layer | Every task receives hard read/edit/tool/token limits |
| Pipeline physics | Tasks move through deterministic stages |
| State machine | Intake, scope, retrieval, assembly, execution, validation, trace |
| Error model | Budget overflow, scope drift, forbidden paths, missing tests |
| Memory layer | Repo rules, decisions, bugs and architectural intent persist |
| Traceability | Each context package and agent action can be audited |
| Minimalism | Start with budget + contract + context assembly before deeper graph work |

---

## 4. Core rule

No agent may read the full repository unless a task contract explicitly allows it.

Default behavior must be:

```text
small task -> small context -> bounded patch -> stop
```

Not:

```text
small task -> autonomous repo exploration -> huge context -> token burn
```

---

## 5. Target architecture

```text
codeaware-mcp
  src/
    kernel/          persistent orchestration and repo truth
    contracts/       task contracts, scope rules, allowed/forbidden paths
    budget/          token, file-read and tool-call budgets
    indexer/         file manifests, hashes, summaries, future parser hooks
    graph/           symbol/dependency graph model
    memory/          architecture memory, decision records, previous bugs
    context/         task context assembly and compression
    trace/           task traces, selected context, validation results
    tools/           MCP tools exposed to agents
```

The existing v3 behavior should remain available. v4 should be added as a new controlled layer first.

---

## 6. v4 pipeline

```text
1. Intake
   Capture user intent and requested outcome.

2. Scope Contract
   Create hard boundaries: files, paths, tokens, tool calls, edit limits.

3. Repo Snapshot
   Identify git state, file hashes and existing cached summaries.

4. Retrieval
   Select relevant files/symbols/tests/configs without full repo reading.

5. Architecture Rules
   Load module rules, project conventions and forbidden patterns.

6. Impact Analysis
   Identify affected files, tests and risky dependencies.

7. Context Assembly
   Build one compact task context package.

8. Agent Execution
   Agent edits within contract only.

9. Validation
   Check scope, tests, diff size, forbidden changes and budget usage.

10. Trace Persist
   Store decision, context package metadata, diff summary and lessons learned.
```

---

## 7. Initial MCP tools

The v4 MCP surface should start small and controlled.

### Required MVP tools

```text
codeaware.create_task_contract
codeaware.check_budget
codeaware.get_task_context
codeaware.record_decision
codeaware.record_change
```

### Next tools

```text
codeaware.repo_map
codeaware.find_symbol
codeaware.explain_symbol
codeaware.find_callers
codeaware.find_tests
codeaware.get_architecture_rules
codeaware.diff_impact
codeaware.stop_if_budget_exceeded
```

The most important tool is `codeaware.get_task_context`.

Agents should call one context assembly tool instead of performing many independent exploratory calls.

---

## 8. Task contract schema draft

```yaml
task:
  intent: implement_feature
  goal: add persistent memory compaction
  scope:
    max_files_read: 8
    max_files_changed: 4
    max_tool_calls: 12
    max_context_tokens: 30000
  allowed_paths:
    - src/memory/**
    - src/context/**
    - tests/memory/**
  forbidden_paths:
    - target/**
    - node_modules/**
    - vendor/**
    - .git/**
  stop_condition:
    - show_diff
    - wait_for_human
```

---

## 9. Context package schema draft

```json
{
  "task_id": "uuid",
  "repo": "owner/name",
  "git_ref": "master",
  "budget": {
    "max_context_tokens": 30000,
    "estimated_context_tokens": 18400,
    "max_files_read": 8,
    "selected_files": 5
  },
  "selected_context": [
    {
      "type": "file_summary",
      "path": "src/memory/store.rs",
      "reason": "Owns persistence behavior"
    },
    {
      "type": "contract",
      "id": "memory_persistence_rules",
      "reason": "Defines allowed behavior"
    },
    {
      "type": "test_hint",
      "path": "tests/memory_store.rs",
      "reason": "Likely validation target"
    }
  ],
  "excluded_context": [
    {
      "path": "target/**",
      "reason": "Generated build output"
    }
  ]
}
```

---

## 10. v4 implementation phases

### Phase 1: Token protection layer

Goal: reduce token burn immediately without a full rewrite.

Deliverables:

- task contract model,
- budget model,
- read-once cache,
- file summary cache,
- `get_task_context`,
- `check_budget`,
- trace log for selected/excluded context.

### Phase 2: Symbol intelligence

Goal: replace file-level exploration with symbol-level retrieval.

Deliverables:

- tree-sitter based symbol extraction,
- symbol graph,
- import/dependency graph,
- `find_symbol`,
- `find_callers`,
- `find_tests`.

### Phase 3: Architecture memory

Goal: make repository intent persistent.

Deliverables:

- module rules,
- architecture decisions,
- known bug memory,
- allowed/forbidden patterns,
- project-specific agent rules.

### Phase 4: Agent router

Goal: route expensive and cheap work to the right model/tool.

Deliverables:

- Claude for hard architecture/review,
- Gemini/Kimi/Qwen for broad or cheaper execution,
- local model support for summaries,
- spend/budget policy.

---

## 11. Success metrics

v4 is successful when:

- common tasks need less than 30k context tokens,
- file reads are bounded by contract,
- repeated file reads are avoided,
- agents stop after a bounded patch,
- context selection is explainable,
- `/compact` recovery does not require a new repo scan,
- and Claude Code can be used as a premium patch executor instead of a repo explorer.

---

## 12. First implementation target

Build v4 as a layer over the existing codeaware-mcp implementation.

Do not rewrite everything.

Start with:

```text
contracts + budget + context package + trace
```

Then attach symbol graph and architecture memory incrementally.
