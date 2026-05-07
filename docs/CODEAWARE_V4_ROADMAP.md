# CodeAware v4 Roadmap

## Vision

Transform CodeAware from a search/compression MCP into a persistent code intelligence kernel.

---

# Phase 1 — Budget & Contract Kernel

## Goal

Prevent uncontrolled repository exploration and token explosion.

## Deliverables

- Task contract engine
- Context budget engine
- Read-once cache
- File summary cache
- Context package builder
- Trace logs
- MCP tools:
  - get_task_context
  - create_task_contract
  - check_budget

## Priority

CRITICAL

## Expected impact

- Immediate token reduction
- Better Claude Code stability
- Smaller Gemini sessions
- Fewer repeated file reads

---

# Phase 2 — Symbol Intelligence

## Goal

Move from file retrieval to symbol retrieval.

## Deliverables

- Tree-sitter indexing
- Symbol graph
- Dependency graph
- Call graph
- Test graph
- MCP tools:
  - find_symbol
  - explain_symbol
  - find_callers
  - find_tests

## Priority

HIGH

## Expected impact

- Major reduction in unnecessary file reads
- Faster task localization
- Better impact analysis

---

# Phase 3 — Architecture Memory

## Goal

Persist architectural intent and project rules.

## Deliverables

- Module rules
- Decision memory
- Known bug memory
- Forbidden patterns
- Allowed patterns
- Team/project constraints

## Example

```yaml
module: auth
rules:
  - never bypass device binding
  - all protected routes require X-Device-Id
```

## Priority

HIGH

## Expected impact

- Less architecture drift
- Better long-term consistency
- Reduced repeated explanations to agents

---

# Phase 4 — Agent Routing

## Goal

Use the right model for the right task.

## Deliverables

- Cheap execution routing
- Premium review routing
- Local model support
- Spend policy layer
- Model capability registry

## Example routing

| Task | Model |
|---|---|
| architecture review | Claude |
| repo exploration | Gemini |
| cheap patching | Qwen/Kimi |
| summaries | local model |

---

# Phase 5 — Persistent Semantic Memory

## Goal

Allow AI coding agents to maintain long-term repository intelligence.

## Deliverables

- Semantic repo memory
- Historical trace search
- Previous patch reasoning
- Change lineage
- Architectural evolution map

---

# Long-Term Direction

CodeAware v4 should evolve toward:

```text
Persistent Code Intelligence Operating Layer
```

instead of:

```text
AI helper tool
```

The system should eventually:

- control context,
- route models,
- manage budgets,
- persist architecture knowledge,
- and orchestrate AI coding execution.
