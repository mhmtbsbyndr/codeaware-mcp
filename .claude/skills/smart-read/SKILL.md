---
name: smart-read
description: Anleitung zur Nutzung des smart_read MCP-Tools.
user-invocable: false
---

## Wann bevorzugt nutzen
- Dateien > 50 LOC
- Wenn Session-Tracking gewünscht ist
- Wenn Symbole, Callers oder Impact-Info benötigt wird

## Wann Standard-Tools okay sind
- Kleine Dateien (< 50 LOC)
- Exakter Rohtext benötigt
- Wenn CodeAware-Tool einen Fehler liefert

## Modi
- auto: Automatische Auswahl basierend auf Dateigröße und Focus
- skeleton: Nur Signaturen und Struktur
- focused: Bestimmtes Symbol oder Zeilenbereich
- full: Vollständiger Inhalt
- diff: Änderungen seit letztem Read
