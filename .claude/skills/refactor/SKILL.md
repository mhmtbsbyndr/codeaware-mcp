---
name: refactor
description: Safe project-wide refactoring with smart_refactor. Rename symbols, preview changes before applying.
argument-hint: "[rename old_name new_name]"
effort: high
context: fork
agent: code-analyzer
user-invocable: true
---

Refaktoriere sicher: $ARGUMENTS

## Workflow

### 1. Codebase verstehen
- Rufe `project_map` auf mit task_context="refactor: $ARGUMENTS"
- Identifiziere die betroffenen Module und Abhängigkeiten
- Verstehe die Architektur bevor du Änderungen planst

### 2. Referenzen finden
- Nutze `smart_read` mode=focused auf die Datei(en), die das Symbol definieren
- Nutze `smart_read` mode=skeleton auf alle Dateien, die das Symbol referenzieren
- Erstelle eine vollständige Liste aller Fundstellen:
  - Definition (struct, fn, trait, type alias)
  - Direkte Aufrufe / Verwendungen
  - Imports und Re-Exports
  - Tests und Dokumentation
  - Konfigurationsdateien (Cargo.toml, etc.)

### 3. Dry-Run Preview
- Rufe `smart_refactor` auf mit:
  - `operation`: z.B. "rename"
  - `target`: das alte Symbol
  - `new_value`: der neue Name
  - `dry_run`: true
- Zeige dem User die Preview:
  - Anzahl betroffener Dateien
  - Jede geplante Änderung mit Kontext (3 Zeilen davor/danach)
  - Potenzielle Risiken (z.B. String-Literale, Makros, generierter Code)

### 4. User-Bestätigung abwarten
- NIEMALS ohne explizite Bestätigung fortfahren
- Bei Bedenken: alternative Vorgehensweise vorschlagen
- User kann einzelne Änderungen ausschließen

### 5. Refactoring anwenden
- Erst nach Bestätigung: `smart_refactor` mit `dry_run`: false
- Prüfe die Ausgabe auf Fehler oder Warnungen
- Bei Teilfehlern: sofort melden, nicht weitermachen

### 6. Verifikation
- `smart_run` mit "cargo test" (Rust) oder äquivalentem Test-Befehl
- `smart_run` mit "cargo clippy" für Lint-Prüfung
- Bei Test-Failures: Ursache analysieren und melden
- Commit-Message vorschlagen bei Erfolg

## Regeln
- IMMER dry_run=true zuerst, NIEMALS direkt anwenden
- Max 3 Iterationen bei Problemen, dann Ergebnis melden
- Keine Änderungen an generierten Dateien (*.generated.*, *.pb.go)
- Bei mehr als 20 betroffenen Dateien: User warnen und Bestätigung einholen
- Impact-Analyse ist Pflicht vor jeder Änderung
