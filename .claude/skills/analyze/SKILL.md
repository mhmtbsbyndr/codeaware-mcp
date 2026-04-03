---
name: analyze
description: Analysiere Projektstruktur und Aufgabe mit minimalen Tokens. Nutzt CodeAware MCP für komprimierte Übersicht.
argument-hint: [aufgabe-oder-frage]
context: fork
agent: code-analyzer
---

Analysiere die Aufgabe: $ARGUMENTS

## Workflow
1. Rufe project_map auf mit task_context="$ARGUMENTS"
2. Identifiziere die 3-5 relevantesten Dateien
3. Nutze smart_read mode=skeleton für jede
4. Erstelle einen konkreten Plan

## Ausgabe
- Betroffene Dateien und Symbole
- Reihenfolge der Änderungen
- Erwartete Tests
- Geschätzte Schritte
