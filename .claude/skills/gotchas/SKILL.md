---
name: gotchas
description: Bekannte Fallstricke und Anti-Patterns bei der Nutzung von CodeAware.
user-invocable: false
---

## Anti-Patterns
1. Dieselbe unveränderte Datei mehrfach lesen → session_status prüfen
2. Edit ohne vorherigen Read → Warnung, aber erlaubt
3. Gleicher Fehler 3x → Error-Loop-Warnung, Root-Cause-Analyse empfohlen
4. project_map vor jedem kleinen Fix → Overhead, direkter Read ist oft besser
5. smart_run für echo/ls → kein Gewinn, native Bash nutzen
