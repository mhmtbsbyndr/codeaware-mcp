---
name: smart-edit
description: Anleitung zur Nutzung des smart_edit MCP-Tools.
user-invocable: false
---

## Strategien
- text: old→new Replace (Standard, Eindeutigkeit erforderlich)
- symbol: Auf Symbol-Ebene editieren (tree-sitter)
- lines: Zeilenbereich ersetzen

## Best Practices
- expected_hash nutzen für Concurrent-Safety
- dry_run=true für Vorschau
- Impact-Analyse beachten (callers_affected, tests_affected)
