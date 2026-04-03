# CodeAware

<important if="reading or editing files">
Bevorzugt CodeAware MCP Tools (smart_read, smart_edit, smart_run) für Dateien > 50 LOC
und verbose Befehle. Standard-Read/Bash bleibt erlaubt für kleine Dateien und einfache Befehle.
</important>

<important if="starting a task">
Bevorzugt /analyze oder project_map als ersten Schritt für unbekannte Codebereiche.
</important>

## Build & Test
- Rust: `cargo test`, `cargo clippy`

## Architektur
@.claude/rules/architecture.md

## Token-Effizienz
@.claude/rules/token-efficiency.md
