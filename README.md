# Fotobuch-Assistent

Arbeitstitel. Ein **lokales, offline arbeitendes Werkzeug**, das die Vorarbeit für ein Fotobuch übernimmt: Fotos ordnen, zu Ereignissen gruppieren, Doppelte und fast Gleiche reduzieren, das beste Bild je Serie vorschlagen und alles auf Seiten planen. Danach exportiert es für den Druckdienst deiner Wahl.

**Stand:** Native Desktop-App (Version 0.2): Rust-Kern, Tauri-Fenster, Oberfläche in HTML/CSS/JS. Geführte Oberfläche in fünf Schritten (Laden, Ereignisse, Aussortieren, Seiten, Export). Siehe [docs/fortschritt.md](docs/fortschritt.md).

## Ausprobieren

Fertige, portable Programme (kein Installer) entstehen in der GitHub-Aktion «App bauen» (`.github/workflows/build-exe.yml`): `fotobuch-assistent.exe` für Windows und `Fotobuch-Assistent-mac.zip` (Apple Silicon) für den Mac. Dazu gibt es ein Linux-Paket (`Fotobuch-Assistent-linux-x86_64.tar.gz`). Veröffentlichte Versionen (Tag `v*`) stehen unter [Releases](https://github.com/goerks/fotobuch-assistent/releases). Webseite (Deutsch/Englisch): Ordner `website/`, gebaut mit `tools/web_demo.py`, `tools/web_screens.py` und `tools/web_build.py`. Anleitung für Nutzer: [docs/handbuch.md](docs/handbuch.md).

Aus dem Quellcode (Rust, aktuelle stabile Version; unter Linux zusätzlich `libwebkit2gtk-4.1-dev`):

```
cargo test -p fotobuch-core              # Kerntests
cargo run --release -p fotobuch-assistent # App mit Fenster
cargo run --release -p fotobuch-core --bin fotobuch-dev -- serve ordner   # nur Server, für Tests im Browser
```

Aufbau: `crates/core` (Logik, Server), `crates/app` (Tauri-Fenster), `ui/` (Oberfläche, wird ins Programm eingebettet), `tests/ui` (Oberflächentests mit jsdom).

## Das Problem

Wer ein Fotobuch bestellt, verliert die meiste Zeit vor dem Gestalten:

1. Fotos in die richtige Reihenfolge bringen
2. Fotos zu Tagen, Ausflügen und Themen gruppieren
3. Doppelte und fast identische Bilder aussortieren, das beste behalten
4. Hoch- und Querformat sinnvoll mischen und zuschneiden
5. Seiten planen: Wie viele Seiten ergibt das? Wie viele Fotos fehlen bis zur nächsten Seitenzahl? Wo passt eine Doppel- oder Panoramaseite?

## Leitplanken

- **Alles lokal und offline.** Keine Fotos verlassen den Rechner, kein Konto, keine Cloud-KI.
- **Vorschlag statt Automatik.** Jede Entscheidung ist erklärbar und umkehrbar, der Mensch bestätigt.
- **Läuft auch auf schwachen Rechnern.** Die Grundfunktionen brauchen keine KI und keine Grafikkarte. KI-Modelle sind optional und klein.
- **Anbieterunabhängig.** Export über dokumentierte Profile pro Druckdienst.
- **Windows zuerst**, danach macOS.
- **Keine Fotos im Repository.** Die `.gitignore` schliesst alle Bild- und RAW-Formate aus.

## Etappen

| Etappe | Inhalt | Richtwert |
|---|---|---|
| 0 Machbarkeit (läuft) | Erkennungsqualität und Tempo auf echtem Windows-Rechner, Annahme der Exportdatei beim Druckdienst | 1 Woche |
| MVP 1 | Ordnen, Ereignisse, exakte Duplikate, Seitenzähler, Export | 2–3 Wochen |
| MVP 2 | Ähnlichkeit, Best-of-Vorschlag, Hoch/Quer-Paarung, Seitenplan, erste Anbieter-Profile | 4–6 Wochen |
| MMP | Installer, Hardware-Stufen, weitere Profile, Test mit Fremden | 6–10 Wochen |

Die Zeiten sind Schätzungen inklusive Abnahmen.

## Dokumentation

- [docs/konzept.md](docs/konzept.md): Problem, Wettbewerb, Differenzierung, Technik, Etappen
- [docs/handbuch.md](docs/handbuch.md): Bedienung für Nutzer
- [docs/pitch/](docs/pitch/): Pitch und Demo vor jedem Testlauf
- [docs/testen.md](docs/testen.md): Testkonzept und bekannte Lücken
- [docs/design.md](docs/design.md): Oberflächen-Richtungen und Entscheidung
- [docs/etappe-0.md](docs/etappe-0.md): Plan der Machbarkeits-Etappe
- [docs/fortschritt.md](docs/fortschritt.md): Fortschrittsprotokoll und offene Entscheidungen
- [profiles/](profiles/): Anbieter-Profile (Entwurf)
- [tests/](tests/): Tests und Berichte

## Lizenz

Noch offen. Wird vor dem ersten Release festgelegt (siehe Entscheidungen im Fortschrittsprotokoll).
