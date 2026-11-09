# Testkonzept

Ziel: Jede Änderung wird automatisch geprüft, ohne echte Fotos. Was automatisch nicht prüfbar ist, steht unten ehrlich als offen.

## Ebenen

| Ebene | Wo | Was wird geprüft |
|---|---|---|
| Kern | `crates/core` (Unit-Tests) und `crates/core/tests/ende_zu_ende.rs` | Analyse, Ereignisse, Duplikate, Serien, Umfangsplanung, Seitenfolge (leere Seiten, Verschieben, Entfernen), Druckdienst-Profile, Entscheidungen, Export |
| Oberfläche | `tests/ui/app.test.mjs` (Node, jsdom) | Die echte App wird gestartet und wie ein Nutzer bedient: Analyse, Ereignisse, Aussortieren mit Reitern und Serienband, Umfang, Druckdienst, Seitenband, Gestaltung, Export |
| Ende-zu-Ende | `tests/e2e/test_album.py` (pytest + Chromium) | Demo-Reise (~300 Fotos) durch die ganze App, Export, Originale unverändert, Neustart behält Änderungen, Bilder je Doppelseite |
| Paket | CI (`build-exe.yml`) | Tests, dann Windows-, Mac- und Linux-Pakete; Selbsttest der exe |

Testbilder sind synthetisch und enthalten bekannte Eigenschaften (Zeit, Unschärfe, Duplikate), damit Erwartungen exakt sind.

## Ausführen
```
cargo test -p fotobuch-core
cargo build --release -p fotobuch-core --bin fotobuch-dev   # vor den Oberflächen- und E2E-Tests
cd tests/ui && npm ci && node --test app.test.mjs
FOTOBUCH_E2E_THEMEN=leinen,nacht python3 -m pytest tests/e2e -q
```

## Bekannte Lücken (offen)
- **Echte Fotos:** Die Qualität der Serien- und Bestenauswahl ist nur an synthetischen Bildern geprüft.
- **Optik:** jsdom rendert nicht. Layout und Lesbarkeit werden über Screenshots angeschaut (Vorher/Nachher-Bilder).
- **Windows und Mac von Hand:** Ordner-Dialog, Fensterstart und Warnhinweise sind nicht von Hand geprüft.
- **RAW:** Nur der Codepfad, keine echte RAW-Datei.
- **Anbieter-Regeln:** Aus Anbieter-Seiten und einem Testbericht (Stand 30.10.2026), nicht aus den Editoren.
