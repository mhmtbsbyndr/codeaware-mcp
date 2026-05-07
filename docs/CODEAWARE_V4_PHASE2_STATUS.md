# CodeAware v4 Phase 2 Status

**Status:** Semantic graph foundation implemented  
**Target:** Move from file-level context to symbol-level semantic retrieval.

---

## Implemented Phase 2 modules

```text
src/v4/symbols.rs
src/v4/ast.rs
src/v4/import_graph.rs
src/v4/call_graph.rs
src/v4/tests_graph.rs
src/v4/impact.rs
src/v4/retrieval.rs
src/v4/semantic_index.rs
src/v4/index_builder.rs
```

---

## Current semantic pipeline

```text
Repository
→ CandidateDiscovery
→ Rust AST parse
→ Symbol extraction
→ Import extraction
→ Call extraction
→ Test extraction
→ SemanticIndex
→ SemanticRetrieval
→ Impact foundation
```

---

## Implemented capabilities

- tree-sitter Rust parsing
- Rust function/struct/enum/trait/module/const symbol extraction
- SymbolIndex with name search
- ImportGraph with Rust use/mod extraction
- CallGraph with heuristic caller/callee extraction
- TestGraph with simple Rust test reference extraction
- ImpactAnalyzer with risk scoring
- SemanticRetrieval for symbols and tests
- SemanticIndex aggregate
- SemanticIndexBuilder for repo-level semantic compilation
- Persistable semantic index JSON

---

## Design transition

Before Phase 2:

```text
Agent → File summaries → ContextPackage
```

After Phase 2 foundation:

```text
Agent → SemanticIndex → Symbols/Imports/Calls/Tests/Impact → Minimal semantic ContextPackage
```

---

## Next implementation targets

1. Symbol-aware `get_task_context`
2. Persistent semantic index under `.codeaware/v4/index/semantic_index.json`
3. MCP tools:
   - `codeaware.find_symbol`
   - `codeaware.find_callers`
   - `codeaware.find_tests`
   - `codeaware.diff_impact`
4. Architecture memory:
   - module rules
   - decision records
   - known bug memory
5. Semantic compact recovery:
   - restore task context from trace + semantic index

---

## Important limitation

The current CallGraph is heuristic. It is useful as an MVP, but should later be replaced or augmented with AST-aware call extraction.

---

## Core principle

The repository should be compiled into persistent semantic memory before expensive LLM reasoning begins.
