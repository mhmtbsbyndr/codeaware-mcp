# CodeAware v4 MCP Tools

## Overview

CodeAware v4 exposes semantic repository intelligence APIs intended for MCP-based AI coding agents.

The v4 runtime is designed around:

```text
bounded semantic context
instead of
uncontrolled repository scanning
```

---

# Semantic-first context

## `codeaware.get_task_context`

Builds a bounded semantic context package for an AI coding task.

### Purpose

Instead of scanning the repository directly, the agent requests a context package assembled from:

- semantic symbols,
- imports,
- calls,
- tests,
- summaries,
- budgets,
- and architecture rules.

### Example request

```json
{
  "name": "codeaware.get_task_context",
  "arguments": {
    "goal": "Refactor context package assembly",
    "intent": "Refactor"
  }
}
```

---

# Symbol retrieval

## `codeaware.find_symbol`

Searches semantic symbols from the persistent semantic index.

### Example

```json
{
  "name": "codeaware.find_symbol",
  "arguments": {
    "repo_root": "/workspace/project",
    "query": "build_context"
  }
}
```

---

# Caller graph lookup

## `codeaware.find_callers`

Returns callers of a semantic symbol.

### Example

```json
{
  "name": "codeaware.find_callers",
  "arguments": {
    "repo_root": "/workspace/project",
    "symbol": "AuthService.login"
  }
}
```

---

# Test lookup

## `codeaware.find_tests`

Returns tests associated with a semantic symbol.

### Example

```json
{
  "name": "codeaware.find_tests",
  "arguments": {
    "repo_root": "/workspace/project",
    "symbol": "ContextPackage"
  }
}
```

---

# Impact analysis

## `codeaware.diff_impact`

Returns estimated semantic impact for a changed file.

### Example

```json
{
  "name": "codeaware.diff_impact",
  "arguments": {
    "repo_root": "/workspace/project",
    "changed_path": "src/v4/tools.rs"
  }
}
```

---

# Semantic execution model

```text
Repository
→ SemanticIndexBuilder
→ SemanticIndex
→ SemanticContextAssembler
→ ContextPackage
→ Agent
```

The agent should prefer semantic APIs over raw repository reads whenever possible.
