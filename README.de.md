# 🧠 codeaware-mcp

<p align="center">
  <strong>Lokale KI-Code-Intelligenz, Token-Kompression, Progressive Memory, semantische Repository-Runtime & Qualitätslayer für MCP-Agenten</strong>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?style=for-the-badge&logo=rust" />
  <img alt="MCP" src="https://img.shields.io/badge/MCP-JSON--RPC-blue?style=for-the-badge" />
  <img alt="Local First" src="https://img.shields.io/badge/Local--First-Ja-success?style=for-the-badge" />
  <img alt="Status" src="https://img.shields.io/badge/Status-v4%20Semantic%20Runtime-purple?style=for-the-badge" />
</p>

---

## 🚀 Was ist codeaware-mcp?

`codeaware-mcp` ist eine **lokale MCP-Runtime** für KI-Coding-Agenten wie Claude Code, Codex-ähnliche Agenten, Cursor/OpenCode-Workflows, Gemini-CLI-ähnliche Agenten und lokale LLM-Workflows.

Das System sitzt zwischen Agent und Repository und liefert **komprimierte, strukturierte, nachvollziehbare Code-Intelligenz** statt roher Dateien, lauter Terminalausgaben, wiederholter Diffs und unkontrolliertem Tokenverbrauch.

Kurz gesagt:

> **Eine lokale Persistent Code Intelligence Runtime mit stabiler Kompressionsbasis und einem v4 Semantic Context Kernel für begrenzte, kontrollierte KI-Coding-Agenten.**

Die Kernidee:

```text
Nicht das LLM soll den Repository-Kontext besitzen.
CodeAware soll ihn besitzen.
```

---

## 🧠 Warum dieses Projekt existiert

Moderne KI-Coding-Tools sind sehr leistungsfähig, arbeiten aber oft noch nach einem teuren Muster: Sie laden wiederholt große Teile des Repositories in das Kontextfenster des Modells.

Das erzeugt fünf Probleme:

1. **Tokenverschwendung** — dieselben Dateien werden wiederholt gelesen.
2. **Kontext-Drift** — nach Kompaktierung verliert das Modell den Grund, warum Dateien wichtig waren.
3. **Schwache Nachvollziehbarkeit** — oft ist unklar, warum eine Datei ausgewählt wurde.
4. **Zu wenig Task-Grenzen** — Agenten explorieren zu viel, statt begrenzt zu patchen.
5. **Keine persistente Repository-Semantik** — jede Session muss das Projekt aus Rohtext neu verstehen.

CodeAware v4 setzt eine Ebene tiefer an:

```text
Das LLM soll das Repository nicht jedes Mal neu entdecken.
Das Repository soll vorher in wiederverwendbare semantische Intelligenz kompiliert werden.
```

---

## 🧠 CodeAware v4 Kernel

CodeAware v4 ergänzt eine persistente semantische Repository-Schicht. Ziel ist es, Tokenverschwendung, unkontrollierte Repo-Scans und wiederholte Kontext-Rehydration zu reduzieren.

### v4-Ausführungsmodell

```text
Repository
→ Discovery
→ AST Parsing
→ Semantic Extraction
→ SemanticIndex
→ SemanticContextAssembler
→ ContextPackage
→ Agent
→ Trace
→ Recovery
→ Architecture Memory
→ Semantic Routing
```

### Implementierte v4-Runtime-Module

```text
src/v4/
  architecture_memory.rs
  budget.rs
  cache.rs
  cache_invalidation.rs
  call_graph.rs
  context.rs
  context_items.rs
  contracts.rs
  discovery.rs
  errors.rs
  impact.rs
  import_graph.rs
  index_builder.rs
  language_support.rs
  precision.rs
  ranking.rs
  recovery.rs
  retrieval.rs
  semantic_context.rs
  semantic_index.rs
  semantic_router.rs
  semantic_tools.rs
  storage.rs
  summaries.rs
  symbols.rs
  tests_graph.rs
  tokens.rs
  tools.rs
  trace.rs
```

### v4-Funktionen

| Funktion | Status |
|---|---|
| Task Contracts | Implementiert |
| Budget Engine | Implementiert |
| Candidate Discovery | Implementiert |
| Ranking | Implementiert |
| Summary-first Fallback | Implementiert |
| Token-Schätzung | Implementiert |
| Context Packages | Implementiert |
| JSONL Trace Persistence | Implementiert |
| tree-sitter Rust AST Parsing | Implementiert |
| Symbol Extraction | Implementiert |
| Import Graph | Implementiert |
| Call Graph Foundation | Implementiert |
| Test Graph Foundation | Implementiert |
| Impact Analysis Foundation | Implementiert |
| SemanticIndex | Implementiert |
| SemanticContextAssembler | Implementiert |
| Semantic-first `get_task_context` | Implementiert |
| Semantic Tools: find_symbol/find_callers/find_tests/diff_impact | Implementiert und im MCP Dispatcher verdrahtet |
| Architecture Memory | Foundation implementiert |
| Decision Memory | Foundation implementiert |
| Semantic Recovery | Foundation implementiert |
| Semantic Router | Foundation implementiert |
| Cache Invalidation | Foundation implementiert |
| Multi-Language Detection | Foundation implementiert |
| Precision Metrics | Foundation implementiert |

---

## 🔌 v4 MCP Tools

Die folgenden v4-Tools sind im MCP `tools/call` Dispatcher verdrahtet:

```text
codeaware.get_task_context
codeaware.find_symbol
codeaware.find_callers
codeaware.find_tests
codeaware.diff_impact
```

### `codeaware.get_task_context`

Erstellt ein begrenztes semantisches Context Package für eine KI-Coding-Aufgabe.

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "codeaware.get_task_context",
    "arguments": {
      "repo_root": "/workspace/project",
      "goal": "Refactor semantic context assembly"
    }
  }
}
```

### `codeaware.find_symbol`

Findet Symbole im semantischen Index.

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "tools/call",
  "params": {
    "name": "codeaware.find_symbol",
    "arguments": {
      "repo_root": "/workspace/project",
      "query": "ContextPackage"
    }
  }
}
```

### `codeaware.find_callers`

Findet Aufrufer eines Symbols.

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "tools/call",
  "params": {
    "name": "codeaware.find_callers",
    "arguments": {
      "repo_root": "/workspace/project",
      "symbol": "build_context"
    }
  }
}
```

### `codeaware.find_tests`

Findet Tests, die zu einem Symbol gehören.

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "tools/call",
  "params": {
    "name": "codeaware.find_tests",
    "arguments": {
      "repo_root": "/workspace/project",
      "symbol": "ContextPackage"
    }
  }
}
```

### `codeaware.diff_impact`

Schätzt die semantischen Auswirkungen einer geänderten Datei.

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "tools/call",
  "params": {
    "name": "codeaware.diff_impact",
    "arguments": {
      "repo_root": "/workspace/project",
      "changed_path": "src/v4/tools.rs"
    }
  }
}
```

---

## ⚖️ Vergleich mit KI-Coding-Tools

CodeAware will KI-Coding-Agenten nicht ersetzen. CodeAware ist die **semantische Kontextschicht unter diesen Agenten**.

| Tool | Hauptrolle | Stärke | Problem, das CodeAware adressiert |
|---|---|---|---|
| Claude Code | Premium-Coding-Agent | Starkes Reasoning und Patch-Ausführung | Kann Kontext/Tokens durch Repo-Exploration verbrennen |
| Cursor | KI-IDE | Schnelles Inline-Coding und gute Editor-UX | Nutzung kann bei großen Kontexten stark steigen |
| Gemini CLI | Budgetfreundlicher Terminal-Agent | Lange Kontexte und breite Exploration | Braucht begrenzte, repo-bewusste Kontextauswahl |
| OpenCode | Offene Agent-Shell | Flexibles lokales/remote Model-Routing | Profitiert ebenfalls von semantischer Repo-Memory |
| Qwen/Kimi/lokale Modelle | Günstige Ausführung und Reviews | Niedrige Kosten für Routineaufgaben | Brauchen kuratierten Kontext, um akkurat zu bleiben |
| CodeAware v4 | Persistente semantische Kontext-Runtime | Kontrolliert Kontext, Budgets, Traces und semantische Suche | Benötigt weiterhin Agenten/Modelle für Reasoning und Patches |

Bestes Setup:

```text
Cursor / Claude Code / Gemini CLI / OpenCode
        ↓
CodeAware MCP
        ↓
SemanticIndex + ContextPackage + Budget + Trace
        ↓
Repository
```

---

## 🧩 Typische Anwendungsfälle

### 1. Claude-Code-Tokenverbrauch reduzieren

Statt einen Agenten das komplette Repository untersuchen zu lassen:

```text
codeaware.get_task_context(goal="Fix login session handling")
```

Danach bekommt der Agent ein kontrolliertes Context Package.

### 2. Exakt finden, wo ein Symbol liegt

```text
codeaware.find_symbol(query="ContextPackage")
```

So müssen keine irrelevanten Dateien geladen werden.

### 3. Vor einem Refactor Callers finden

```text
codeaware.find_callers(symbol="build_context")
```

Nützlich vor Refactors, Renames und Verhaltensänderungen.

### 4. Zugehörige Tests finden

```text
codeaware.find_tests(symbol="ContextPackage")
```

Nützlich für minimale Testauswahl.

### 5. Auswirkungen einer Änderung schätzen

```text
codeaware.diff_impact(changed_path="src/v4/tools.rs")
```

Hilfreich vor Commits oder riskanten Änderungen.

---

## 🧱 Design-Prinzipien

### 1. Kontext gehört der Runtime

Das Modell soll nicht frei entscheiden, wie viel Repository es liest.

```text
Agent fragt Kontext an.
CodeAware entscheidet, welcher Kontext erlaubt ist.
```

### 2. Semantik zuerst, Datei-Summary danach

CodeAware versucht zuerst semantischen Kontext:

```text
symbols → imports → calls → tests → impact
```

Nur wenn kein semantischer Kontext verfügbar ist, fällt es auf Datei-Summaries zurück.

### 3. Begrenzte Ausführung

Jede Aufgabe sollte Limits haben:

```text
max files read
max files changed
max tool calls
max context tokens
stop conditions
```

### 4. Alles ist nachvollziehbar

Jedes Context Package sollte erklärbar sein:

```text
Warum wurde diese Datei ausgewählt?
Warum wurde dieser Pfad ausgeschlossen?
Wie viele Tokens wurden geschätzt?
```

### 5. Local-first als Standard

Repository-Intelligenz soll lokal verfügbar sein, ohne das komplette Codebase an externe Dienste zu senden.

---

## 🧪 Ehrlicher Status

Die v4-Architektur, Runtime-Module, semantischen APIs und MCP-Dispatcher-Verdrahtung sind implementiert.

Ein Repository ist jedoch erst dann wirklich produktionsreif, wenn CI/Build verifiziert wurden.

Aktuelle Wahrheit:

```text
Implementiert: ja
Dokumentiert: ja
MCP-Dispatcher verdrahtet: ja
CI Workflow vorhanden: ja
CI grün: muss über GitHub Actions nach Workflow-Ausführung geprüft werden
```

Lokal prüfen:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
cargo build --release --all-features
```

---

## ⚡ Schnellstart

### 1. Repository klonen

```bash
git clone https://github.com/mhmtbsbyndr/codeaware-mcp.git
cd codeaware-mcp
```

### 2. MCP-Server bauen

```bash
cargo build --release
```

Das Binary liegt danach hier:

```bash
./target/release/codeaware-mcp
```

### 3. Tests ausführen

```bash
cargo test
```

### 4. Lokal über stdio starten

```bash
./target/release/codeaware-mcp
```

Der Server spricht **JSON-RPC über stdio**, wie von MCP-Clients erwartet.

### 5. In Claude Code / MCP konfigurieren

Beispiel `.mcp.json`:

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

Für MCP-Clients sollte immer ein absoluter Pfad verwendet werden.

---

## ✅ Anforderungen

| Voraussetzung | Version / Hinweise |
|---|---|
| Rust | kompatibel mit Edition 2021 |
| Cargo | Teil der Rust-Installation |
| SQLite | genutzt von bestehender Session-/Memory-Basis |
| Git | erforderlich für Git-Intelligence-Tools |
| Claude Code oder MCP-Client | jeder Client mit stdio-MCP-Unterstützung |

Empfohlen:

```bash
rustup update
cargo build --release
cargo test
```

---

## 🧭 Warum v4 wichtig ist

KI-Coding-Agenten verschwenden häufig Kontext auf:

```text
Read("src/server.rs")       -> hunderte Quellcodezeilen
Run("cargo test")           -> laute Log-Ausgaben
Read("src/server.rs") again -> derselbe Inhalt erneut
Context compaction           -> Arbeitsgedächtnis verschwindet
```

CodeAware v4 bewegt sich in Richtung:

```text
codeaware.get_task_context   -> begrenztes semantisches Context Package
codeaware.find_symbol        -> Symbol-Level Retrieval
codeaware.find_callers       -> Caller-Graph Lookup
codeaware.find_tests         -> relevante Tests
codeaware.diff_impact        -> impact-bewusstes Reasoning
semantic_router              -> Cheap/Balanced/Premium Model-Routing-Hinweis
semantic_recovery            -> kompakter Task-Recovery-Snapshot
```

Das Ziel sind nicht nur weniger Tokens.

Das Ziel ist **besserer, dichterer und begrenzter semantischer Kontext**.

---

## 🏗️ Architektur

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
      +-- v4 Persistent Code Intelligence Kernel
      |     +-- task contracts
      |     +-- budget engine
      |     +-- discovery/ranking
      |     +-- summaries/token estimation
      |     +-- context packages
      |     +-- semantic index
      |     +-- symbols/imports/calls/tests
      |     +-- impact analysis
      |     +-- architecture memory
      |     +-- semantic recovery
      |     +-- semantic router
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
      +-- Progressive Memory Foundation
      |     +-- compact memory index
      |     +-- timeline window
      |     +-- observation details
      |     +-- privacy tag filtering
      |     +-- memory citations
      |
      +-- Safety Foundation
            +-- security policy
            +-- command validation
            +-- path validation
            +-- MCP routing
```

---

## 🧪 Server prüfen

Tests ausführen:

```bash
cargo test
```

Binary manuell starten:

```bash
./target/release/codeaware-mcp
```

Beispiel für JSON-RPC Initialize:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

Beispiel für existierenden Tool-Call:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"token_stats","arguments":{}}}
```

Beispiel für v4 Semantic Tool Call:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"codeaware.get_task_context","arguments":{"repo_root":".","goal":"Explain v4 semantic context"}}}
```

---

## 🗂️ v4-Dokumentation

Die v4-Architektur ist dokumentiert in:

```text
docs/CODEAWARE_V4_MASTERPLAN.md
docs/CODEAWARE_V4_ROADMAP.md
docs/CODEAWARE_V4_PHASE1_IMPLEMENTATION_SPEC.md
docs/CODEAWARE_V4_IMPLEMENTATION_SNAPSHOT.md
docs/CODEAWARE_V4_PHASE2_STATUS.md
docs/CODEAWARE_V4_FINAL_ARCHITECTURE.md
docs/CODEAWARE_V4_MCP_TOOLS.md
```

---

## 🧭 About dieser v4-Release

`codeaware-mcp` wird mit dem `v4`-Milestone als eigenständiger Release-Zweig ausgeliefert.

- Kontextbasierte Paketierung ist der Standardpfad für semantische V4-Workflows.
- Kontext-Budgets, semantische Suche, Traces und Recovery sind für kontrollierte, leise Agenten-Workflows zusammengeführt.
- Die v4-MCP-Tools sind über `tools/call` vollständig verfügbar: `get_task_context`, `find_symbol`, `find_callers`, `find_tests`, `diff_impact`.
- Persistenter semantischer Zustand und Architektur-Memory-Primitiven sind in diesem Release aktiv.
- GitHub Release-Tag: `v4.0.0` (erstes öffentliches v4-Milestone).

---

## 🚦 Aktueller Status

CodeAware hat aktuell:

- echte Rust-Crate-Struktur,
- echten MCP-stdio-Server,
- echten JSON-RPC-Dispatch für bestehende Tools,
- v4 Semantic MCP Tool Dispatch,
- stabile kompressionsorientierte MCP-Tools,
- Runtime-verdrahtete Token-/Quality-/Benchmark-/Context-Tools,
- Progressive-Memory-Grundlagen,
- v4 Semantic-Code-Intelligence-Runtime-Grundlagen,
- semantic-first Context Assembly APIs,
- persistentes Semantic-Repository-Kernel-Design.

Verbleibende Production-Hardening-Aufgaben:

```text
- vollständige cargo tests in CI/lokaler Umgebung ausführen
- mögliche Compile-/Test-Regressions beheben
- tree-sitter Support über Rust hinaus erweitern
- production-grade AST Call Extraction ergänzen
- Semantic-Index-Cache-Invalidation verbessern
```

---

## 🛣️ Roadmap

### Kurzfristig

- GitHub Actions CI grün bestätigen.
- `tools/list` Metadaten für v4 Tools verfeinern.
- Semantic Index Persistence und Cache Invalidation verbessern.
- Benchmarks für Tokenersparnis gegenüber rohen Datei-Reads ergänzen.

### Mittelfristig

- TypeScript/JavaScript/Python/PHP/Go/Swift/Java Extraction ergänzen.
- Heuristischen Call Graph durch AST-bewusste Call Extraction ersetzen.
- Architecture Memory und Decisions persistenter und besser abfragbar machen.
- Semantic Diffing über Commits ergänzen.

### Langfristig

- Multi-Model Routing nach semantischer Komplexität.
- Persistente Cross-Repository-Memory.
- Semantic Task Planner.
- Minimale Testauswahl.
- IDE-/LSP-Integration.

---

## 🧩 Entwicklungsphilosophie

Jedes Feature sollte mindestens eine dieser Kosten reduzieren:

- wiederholtes Kontextlesen,
- laute Terminalausgaben,
- verlorenes Session-Gedächtnis,
- unsichere Edits,
- unklare Code-Auswirkungen,
- nicht überprüfbare KI-Behauptungen,
- Tool-Schema-Overload,
- Cross-Repo-Blindheit,
- Qualitätsverlust durch Überkompression,
- unkontrollierte semantische Drift.

`codeaware-mcp` geht nicht nur um weniger Tokens.

Es geht um **bessere Tokens und persistente semantische Code-Intelligenz**.
