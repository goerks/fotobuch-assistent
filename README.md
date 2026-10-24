# Fotobuch-Assistent

Arbeitstitel. Ein **lokales, offline arbeitendes Werkzeug**, das die Vorarbeit für ein Fotobuch übernimmt: Fotos ordnen, zu Ereignissen gruppieren, Doppelte und fast Gleiche reduzieren, das beste Bild je Serie vorschlagen und alles auf Seiten planen. Danach exportiert es für den Druckdienst deiner Wahl.

**Stand:** Konzeptphase, noch kein lauffähiger Code. Siehe [docs/fortschritt.md](docs/fortschritt.md).

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
| 0 Machbarkeit | Erkennungsqualität und Tempo auf echtem Windows-Rechner, Annahme der Exportdatei beim Druckdienst | 1 Woche |
| MVP 1 | Ordnen, Ereignisse, exakte Duplikate, Seitenzähler, Export | 2–3 Wochen |
| MVP 2 | Ähnlichkeit, Best-of-Vorschlag, Hoch/Quer-Paarung, Seitenplan, erste Anbieter-Profile | 4–6 Wochen |
| MMP | Installer, Hardware-Stufen, weitere Profile, Test mit Fremden | 6–10 Wochen |

Die Zeiten sind Schätzungen inklusive Abnahmen.

## Dokumentation

- [docs/konzept.md](docs/konzept.md): Problem, Wettbewerb, Differenzierung, Technik, Etappen
- [docs/etappe-0.md](docs/etappe-0.md): Plan der Machbarkeits-Etappe
- [docs/fortschritt.md](docs/fortschritt.md): Fortschrittsprotokoll und offene Entscheidungen
- [profiles/](profiles/): Anbieter-Profile (Entwurf)
- [tests/](tests/): Tests und Berichte

## Lizenz

Noch offen. Wird vor dem ersten Release festgelegt (siehe Entscheidungen im Fortschrittsprotokoll).
