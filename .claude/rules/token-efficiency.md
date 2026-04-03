# Token-Effizienz

## Priorisierung
- smart_read bevorzugt für Dateien > 50 LOC
- smart_run bevorzugt für Tests, Builds, Linting
- smart_edit bevorzugt wenn Impact-Analyse gewünscht
- Standard-Tools sind NICHT verboten – nutze sie bei Bedarf

## Session-Hygiene
- session_status nutzen bei Stagnation oder nach /compact
- Subagents für isolierte Teilaufgaben (sauberer Hauptkontext)
- Nicht dieselbe unveränderte Datei mehrfach lesen
