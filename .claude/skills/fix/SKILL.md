---
name: fix
description: Fixe einen Bug mit fokussiertem Lesen und komprimiertem Test-Output.
argument-hint: [bug-beschreibung]
context: fork
agent: bug-fixer
---

Fixe den Bug: $ARGUMENTS

## Workflow
1. project_map → Überblick
2. smart_read mode=auto mit focus auf relevanten Bereich
3. smart_edit → Fix anwenden (strategy=symbol bevorzugt)
4. smart_run → Tests verifizieren
5. Bei Failure: session_status → Was wurde schon versucht?

## Regeln
- Max 5 Iterationen, dann Ergebnis melden
- Impact-Analyse bei jedem Edit prüfen
- Commit-Message vorschlagen bei Erfolg
