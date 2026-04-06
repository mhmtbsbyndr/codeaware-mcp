---
name: coverage
description: Test coverage analysis. Find untested functions and generate a coverage report.
argument-hint: "[language]"
effort: medium
context: fork
agent: code-analyzer
user-invocable: true
---

Coverage-Analyse: $ARGUMENTS

## Workflow

### 1. Coverage Map erstellen
- Rufe `test_coverage_map` auf mit:
  - `language`: aus $ARGUMENTS oder auto-detect aus Projektstruktur
  - Standardwert: "rust" falls Cargo.toml vorhanden
- Analysiere die Ausgabe:
  - Gesamtabdeckung (Prozent)
  - Pro-Datei-Abdeckung
  - Ungetestete Funktionen/Methoden

### 2. Kritische Lücken identifizieren
- Sortiere Dateien nach Coverage (niedrigste zuerst)
- Priorisiere nach Risiko:
  - **Hoch**: Public API, Error Handling, Security-relevanter Code
  - **Mittel**: Business-Logik, Datenverarbeitung
  - **Niedrig**: Utilities, Logging, Config-Parsing
- Für die Top-10 ungetesteten Funktionen:
  - `smart_read` mode=focused auf jede Funktion
  - Verstehe: Was tut die Funktion? Welche Edge Cases gibt es?

### 3. Test-Strategien vorschlagen
Für jede ungetestete Funktion, erstelle einen konkreten Vorschlag:
- **Funktionsname** und Dateipfad
- **Warum testen**: Risiko-Einschätzung
- **Test-Typ**: Unit, Integration, oder Property-Based
- **Test-Szenario**: Konkreter Testfall mit:
  - Input-Werte (inkl. Edge Cases)
  - Erwartetes Verhalten
  - Error Cases
- **Geschätzter Aufwand**: Klein (< 10 LOC), Mittel (10-30 LOC), Groß (> 30 LOC)

### 4. Coverage-Report ausgeben

#### Zusammenfassung
```
Gesamt-Coverage: XX%
Dateien analysiert: NN
Ungetestete Funktionen: MM
Kritische Lücken: KK
```

#### Top ungetestete Bereiche
Tabelle: Datei | Funktion | Risiko | Empfohlener Test-Typ | Aufwand

#### Empfohlene nächste Schritte
- Priorisierte Liste der zu schreibenden Tests
- Quick Wins (einfache Tests mit hohem Impact)
- Komplexe Tests die Refactoring benötigen

## Regeln
- KEINE Tests schreiben, nur analysieren und empfehlen
- Ergebnis in max 600 Tokens zusammenfassen
- Bei Projekten ohne Test-Framework: Framework-Empfehlung geben
- Coverage-Schwellenwerte:
  - < 40%: CRITICAL
  - 40-70%: WARNING
  - 70-90%: ACCEPTABLE
  - > 90%: GOOD
- Fokus auf testbare Public-API, nicht auf private Hilfsfunktionen
