---
name: git-review
description: AI-powered git analysis. Structured diffs, blame context, and PR-ready changelogs.
argument-hint: "[base-branch]"
effort: medium
context: fork
agent: code-reviewer
user-invocable: true
---

Git-Analyse: $ARGUMENTS

## Workflow

### 1. Strukturierte Diff-Analyse
- Rufe `git_diff` auf mit dem angegebenen Base-Branch (Standard: main)
- Analysiere die strukturierte Ausgabe:
  - Geänderte Dateien nach Kategorie (src, tests, config, docs)
  - Hinzugefügte vs. entfernte Zeilen pro Datei
  - Neue Symbole (Funktionen, Structs, Traits) identifizieren
  - Gelöschte Symbole und deren ehemalige Aufrufer prüfen

### 2. Blame-Kontext für Schlüsseländerungen
- Für die Top-5 geänderten Dateien (nach Diff-Größe):
  - `git_blame` aufrufen für die geänderten Bereiche
  - Verstehe: Wer hat den Code zuletzt geändert und warum?
  - Identifiziere: Wurde kürzlich geänderter Code erneut angefasst? (Churn-Indikator)
  - Prüfe: Gibt es mehrere Autoren im selben Bereich? (Merge-Risiko)

### 3. Changelog generieren
- Rufe `git_changelog` auf für den Branch-Bereich
- Prüfe die kategorisierte Ausgabe:
  - Features (feat:)
  - Bug Fixes (fix:)
  - Refactoring (refactor:)
  - Breaking Changes (BREAKING:)
  - Sonstige (chore:, docs:, test:)

### 4. Ergebnis präsentieren

#### Änderungsübersicht
- Tabelle: Datei | Typ | +/- Zeilen | Risiko (low/med/high)
- Gesamtstatistik: Dateien, Zeilen hinzugefügt/entfernt

#### Historischer Kontext (aus Blame)
- Warum wurden die Bereiche ursprünglich geschrieben?
- Gibt es wiederkehrende Änderungen an denselben Stellen?
- Relevante vorherige Commits auflisten

#### PR-Summary
- Titel-Vorschlag (max 70 Zeichen)
- Body mit:
  - ## Summary (3-5 Bullet Points)
  - ## Changes (kategorisiert)
  - ## Risk Assessment (basierend auf Blame-Kontext)
  - ## Test Plan (was muss getestet werden)

## Regeln
- Nur Analyse, KEINE Edits durchführen
- Bei mehr als 50 geänderten Dateien: Zusammenfassung auf Modul-Ebene
- Blame nur für die wichtigsten Änderungen abrufen (Token-Effizienz)
- Ergebnis in max 800 Tokens zusammenfassen
- Bei Merge-Konflikten oder problematischen Bereichen: explizit warnen
