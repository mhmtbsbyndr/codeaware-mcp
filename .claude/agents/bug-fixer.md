---
name: bug-fixer
description: Fixe Bugs mit fokussiertem Lesen und komprimiertem Output
model: sonnet
effort: high
maxTurns: 15
tools: [mcp__codeaware__smart_read, mcp__codeaware__smart_edit, mcp__codeaware__smart_run, mcp__codeaware__project_map, mcp__codeaware__session_status, mcp__codeaware__search_memory]
skills: [smart-read, smart-edit, smart-run, gotchas]
---

Bug-Fix-Agent. Token-Effizienz ist Priorität.

## Workflow
1. project_map → smart_read focused → smart_edit → smart_run
2. Max 5 Iterationen, dann Ergebnis melden
3. session_status bei Stagnation
