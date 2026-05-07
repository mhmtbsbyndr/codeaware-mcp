# CodeAware v4 Final Architecture

## Mission

Transform AI coding from repeated repository scanning into persistent semantic repository intelligence.

---

# Core architecture

```text
Repository
→ Discovery
→ AST Parsing
→ Semantic Extraction
→ SemanticIndex
→ SemanticContextAssembler
→ ContextPackage
→ Agent
```

---

# Runtime layers

## Foundation Layer

- Contracts
- Budget Engine
- Discovery
- Ranking
- Summaries
- Token Estimation
- Context Packages
- Trace Persistence

## Semantic Layer

- AST Parsing
- Symbol Extraction
- Import Graph
- Call Graph
- Test Graph
- Impact Analysis
- Semantic Retrieval
- Semantic Context Assembly
- Persistent Semantic Index

## Intelligence Layer

- Architecture Memory
- Decision Memory
- Semantic Recovery
- Semantic Router
- Complexity-based Model Selection

---

# Key principle

The LLM must not own repository context.

CodeAware v4 owns:

- context selection,
- semantic retrieval,
- budget control,
- semantic persistence,
- traceability,
- architectural memory,
- and semantic recovery.

---

# Semantic-first execution

Before:

```text
Agent
→ Repo scan
→ Large prompt
→ Expensive reasoning
```

After:

```text
Agent
→ SemanticIndex
→ Minimal semantic context
→ Focused reasoning
```

---

# Implemented runtime modules

```text
architecture_memory
budget
cache
call_graph
context
context_items
contracts
discovery
errors
impact
import_graph
index_builder
ranking
recovery
retrieval
semantic_context
semantic_index
semantic_router
semantic_tools
storage
summaries
symbols
tests_graph
tokens
tools
trace
```

---

# Implemented semantic APIs

```text
find_symbol
find_callers
find_tests
diff_impact
get_task_context
```

---

# Long-term direction

CodeAware v4 should evolve into:

```text
Persistent Code Intelligence Runtime
```

instead of:

```text
an AI coding helper
```

The repository becomes a persistent semantic memory space rather than a repeatedly scanned text corpus.
