---
name: fotobuch-assistent-arbeitsweise
description: Arbeite am Projekt Fotobuch-Assistent (goerks/fotobuch-assistent) nach dem vereinbarten Vorgehensmodell: Design-Vorschlag, Bestätigung, Umsetzung, Tests, Vorher/Nachher-Bild, Doku, CI.
---

# Fotobuch-Assistent: Arbeitsweise

Kurzfassung. Massgeblich sind `CLAUDE.md`, `docs/arbeitsweise.md` und `docs/uebergabe.md` im Repository https://github.com/goerks/fotobuch-assistent.

Zuerst das Repository klonen und diese Dateien lesen. Gespräche und Oberfläche auf Schweizer Hochdeutsch (ss statt ß), kurze Antworten, Links statt Dateipfade (Auftraggeber liest meist auf dem Handy).

## Unverrückbare Regeln

- Lokal und offline, keine Cloud. Originale nie verändern, Fotos nie beschneiden. Läuft auf schwachen PCs. Kern ist Open Source. Portable Builds (Windows exe, Mac zip Apple Silicon, Linux tar.gz).
- Ein Ereignis bevorzugt eine Doppelseite, darf aber eine oder mehrere (auch ungerade) Seiten belegen. Pro Ereignis eine überschreibbare Galerie.
- Lokale KI-Modelle: nur gemerkt (`docs/ideen/lokale-modelle.md`), nicht umsetzen ohne Auftrag.
- Keine echten Fotos ins Repository. Release nur mit ausdrücklicher Zustimmung.

## Ablauf je Schritt

1. Analyse des heutigen Ablaufs.
2. Design-Vorschlag als Board (Vorher links, Erklärung in der Mitte, Nachher rechts). Quellen: `docs/design/canvas/`. Auftraggeber bestätigt, vorher nicht bauen.
3. Direkt umsetzen, am Ende melden.
4. Tests: `cargo test -p fotobuch-core`; Release-Build neu (`cargo build --release -p fotobuch-core --bin fotobuch-dev`); jsdom (`cd tests/ui && node --test app.test.mjs`); E2E (`FOTOBUCH_E2E_THEMEN=leinen,nacht python3 -m pytest tests/e2e`).
5. Vorher/Nachher-Bild mit `tools/vorher_nachher.py` nach `docs/galerie/schritte/NN-name.png`, anschauen.
6. Doku: `docs/fortschritt.md`, `docs/handbuch.md`, Webseitentexte (`tools/web_build.py`).
7. Commit, Push, CI manuell starten (`gh api -X POST repos/goerks/fotobuch-assistent/actions/workflows/build-exe.yml/dispatches -f ref=main`) und Jobs prüfen. Mac-Job bei Abbruch mit `rerun-failed-jobs` wiederholen.
8. Kurzer Bericht: was neu ist, was geprüft wurde und was nicht, Link zum Bild, nächster Schritt.

Release nur nach Zustimmung: CI mit `-f 'inputs[veroeffentlichen]=vX.Y.Z'` starten. Ehrlich sagen, was geprüft wurde, und bei Daten von Dritten Quelle und Prüfdatum nennen.
