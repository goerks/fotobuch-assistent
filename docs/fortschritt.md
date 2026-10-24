# Fortschritt

Wird nach jedem Arbeitsschritt ergänzt. Neueste Einträge oben.

## 2026-10-24: Erste lauffähige App (zweiter Wurf Richtung MVP 1)

**Fertig**
- App mit geführter Oberfläche (Richtung C plus A-Elemente), fünf Schritte: Fotos laden, Ereignisse (umbenennbar, Zeitlücke einstellbar), Aussortieren (Serien per Klick und Tastatur, Duplikate/Screenshots zurückholbar), Seiten (Seitenzähler, Auffüllen/Kürzen mit konkreten Fotos, Seitenregeln einstellbar), Export (nummerierte Ereignis-Ordner, `seitenliste.csv`, `LIESMICH.txt`)
- Lokaler Server nur auf 127.0.0.1 mit Token und Host-Prüfung, Oberfläche ohne externe Ressourcen
- Entscheidungen und Analyse bleiben im Fotoordner (`.fotobuch`) gespeichert, zweiter Start liest nur Neues
- Handbuch ([handbuch.md](handbuch.md)), Testkonzept ([testen.md](testen.md))
- Tests: 34 Python-Tests (Kern und App-Schicht inkl. Sicherheit) und 7 Oberflächentests, die die echte App in jsdom bedienen. Alle grün (Linux)
- Pipeline: CI führt beide Testarten aus, baut die Windows-exe und startet sie als Selbsttest

**Nicht geprüft**
- Windows-exe nie von Hand ausgeführt (Ordner-Dialog, Browserstart, SmartScreen-Hinweis)
- Aussehen im echten Browser nicht angeschaut (kein Browser in der Entwicklungsumgebung); nur Verhalten ist getestet
- Echte Fotos, echtes RAW, Anbieter-Seitenregeln wie bisher offen

**Bewusst noch nicht drin (MVP 2)**
- Buchvorschau mit Doppelseiten, Hoch/Quer-Paarung, Panorama, Seitenexport als JPEG/PDF, Ereignisse verschieben oder zusammenlegen, echte Anbieter-Importdateien

## 2026-10-24: Erste Iteration Richtung MVP 1

**Fertig**
- Lauffähiger Kern (`src/fotobuch/`): Einlesen mit Aufnahmezeit, GPS, Kamera, Format und Drehung; Ereignisse nach Zeitlücke und Ortssprung; exakte Duplikate per Datei-Hash; Serien per Differenzhash mit zeitlich begrenzter Spanne; Vorschlag nach Schärfe und Auflösung; Seitenzähler mit Profilen; Export in nummerierte Ereignis-Ordner (kopiert, Originale bleiben)
- Bericht ohne Bilder: lokal `reihenfolge.csv` und `bericht.md`, teilbar `bericht-teilbar.json` (anonymisierte Namen, keine Pfade, kein GPS, kein Ordnername), Zuordnung getrennt in `zuordnung-NICHT-TEILEN.csv`
- 17 automatische Tests mit synthetischen Bildern, alle bestanden
- Doppelklick-Modus: Ohne Argumente öffnet die `.exe` einen Ordner-Dialog und legt die Berichte im Ordner `fotobuch-bericht` ab
- Build-Pipeline für die Windows-`.exe` (GitHub Actions, PyInstaller). Eine Linux-Probe (43 MB) lief eigenständig
- Drei Oberflächen-Vorschläge zur Auswahl (siehe [design.md](design.md))

**Messwerte (nur Richtwert, Cloud-Umgebung mit 2 Kernen, synthetische 12-MP-JPEGs)**
- Etwa 45 s pro 1000 Fotos für Einlesen und Analyse. Das Ziel von 15 Minuten ist damit auf dieser Hardware weit unterschritten. Echte Werte kommen vom Windows-Test, besonders für HEIC und RAW.

**Gefundene Probleme und Behebung**
- Der Test deckte auf, dass der teilbare Bericht den Ordnernamen enthielt. Behoben, durch Test abgesichert.
- Ähnliche Bilder wurden zu einer Riesen-Serie verkettet. Serien sind jetzt zeitlich begrenzt (Standard 30 Minuten Spanne), durch Test abgesichert.

**Nicht geprüft**
- Die Windows-`.exe` wurde noch nicht gebaut und nie auf Windows ausgeführt.
- HEIC und RAW wurden nur mit erzeugten Testdateien geprüft (HEIC), RAW gar nicht (kein rawpy-Test mit echten RAW-Dateien).
- Die Seitenzahl-Regeln der Anbieter sind Annahmen (24 Seiten, Schritt 2), die Profile für Cewe/Pixum und Ifolor haben noch keine geprüften Werte.
- Die Qualität der Vorschläge ist nur an synthetischen Bildern geprüft, nicht an echten Fotos.

**Entscheidungen des Auftraggebers (heute)**
- Auslieferung als einzelne `.exe` ohne Voraussetzungen
- Formate: JPG, HEIC, RAW, PNG/Screenshots
- Projektgrösse: 500 bis 2000 Fotos; Druckdienste: Cewe/Pixum und Ifolor
- Testgerät: normaler Windows-PC ohne Grafikkarte; Tempo höchstens 15 Minuten pro 1000 Fotos
- Läufe mit echten Fotos lokal, nur der teilbare Bericht geht an den Entwickler
- Lizenz: Open-Source-Kern, Komfort-Version kostenpflichtig

**Offen**
- Oberfläche wählen: A Zeitleiste, B Buch zuerst oder C Assistent
- Genaue Open-Source-Lizenz für den Kern wählen (z. B. Apache-2.0 oder MIT). Bis dahin liegt bewusst keine LICENSE-Datei im Repository
- Windows-`.exe` aus der Pipeline bauen lassen und auf dem Testrechner ausführen
- Testordner, Buchformat und Seitenzahl bei Cewe und Ifolor, RAW-Kameramarke, endgültiger Projektname

## 2026-10-24: Repository aufgesetzt

**Fertig**
- Repository mit Grundstruktur: README, `.gitignore` (keine Bilder, RAW, Modelle oder Datenbanken), `docs/`, `profiles/`, `tests/`
- Konzept v0.2 (Windows zuerst, Export-Strategie, Technik, Risiken)
- Plan für Etappe 0

**Offen**
- Eckdaten vom Auftraggeber: Windows-Rechner, Python-Version, Testordner, Druckdienst mit Buchformat und Seitenzahl
- Test-Skript für Etappe 0 (wird geschrieben, sobald die Eckdaten da sind)

**Entscheidungen für den Auftraggeber**
1. Lizenz des Projekts (offen, privat bleiben oder später veröffentlichen?)
2. Endgültiger Projektname (Arbeitstitel: Fotobuch-Assistent)
3. Schwelle für akzeptable Analysezeit pro 1000 Fotos
4. Erfolgskriterien von Etappe 0 bestätigen

## 2026-10-23/24: Konzeptphase

- Problemverständnis und Wettbewerbsrecherche (drei Lager, keine passende Lösung gefunden)
- Export-Recherche: kein einheitliches Fotobuch-Format, PDF nur bei einem Teil der Dienste
- Technologie-Vorschlag mit drei Leistungsstufen für schwache Rechner
- Entscheidungen des Auftraggebers: Windows zuerst, Schnittstellen nur als Export zu Druckdiensten, Fotos bleiben lokal
