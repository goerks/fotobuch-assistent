# Etappe 0: Machbarkeit

Ziel: Mit echten Fotos auf einem echten Windows-Rechner klären, ob der Ansatz trägt, bevor MVP 1 beginnt. Dauer: etwa 1 Woche.

## Fragen, die beantwortet werden

1. **Ordnen:** Wie zuverlässig liefern EXIF-Zeit und GPS eine richtige Reihenfolge und sinnvolle Ereignisse (mehrere Geräte, fehlende EXIF-Daten)?
2. **Duplikate:** Wie gut finden exakte Hashes und Perceptual Hash Doppelte und Serien? Wie viele Fehltreffer?
3. **Ähnlichkeit mit Modell:** Bringt ein kleines Embedding-Modell (z. B. SigLIP 2 Base auf der CPU) spürbar bessere Serien als der Perceptual Hash?
4. **Tempo:** Wie lange dauert die Analyse pro 1000 Fotos auf einem normalen Windows-Rechner ohne Spezial-Hardware?
5. **Qualitätsmasse:** Trennen Schärfe- und Belichtungsmessung unbrauchbare von brauchbaren Bildern?
6. **Anbieter:** Welche Dateien nimmt der Druckdienst des Auftraggebers an (Format, Beschnitt, Einzel- oder Doppelseiten, Seitenzahl-Schritte)? Funktioniert ein Testexport?

## Ablauf

1. Auftraggeber liefert die Eckdaten (siehe unten).
2. Ich schreibe ein **lokales Test-Skript** (Python), das auf dem Windows-Rechner läuft und nur einen **Bericht** erzeugt.
3. Auftraggeber führt es lokal aus. **Es werden keine Fotos hochgeladen.**
4. Der Bericht enthält nur Zahlen, Gruppenlisten und Dateinamen. Auf dieser Basis werte ich aus.
5. Anbieter-Anforderungen werden aus der Anbieter-Dokumentation zusammengetragen und mit einem Testexport geprüft.
6. Ergebnis: kurzer Bericht in `docs/fortschritt.md` mit Empfehlung für MVP 1.

## Was vom Auftraggeber gebraucht wird

- Windows-Version, Prozessor, Arbeitsspeicher, Grafikkarte
- Python 3.10 oder neuer (`python --version`) oder Bereitschaft zur Installation
- Testordner: etwa 500 bis 2000 Fotos eines echten Projekts mit Serien und Doppelten. Angaben zu Anzahl, Geräten und Formaten (JPG, HEIC, RAW)
- Druckdienst, typisches Buchformat und Seitenzahl

## Erfolgskriterien (Vorschlag, gemeinsam zu bestätigen)

- Reihenfolge und Ereignisse stimmen bei der Stichprobe des Auftraggebers weitgehend.
- Serien werden gefunden, ohne dass viele verschiedene Motive zusammengeworfen werden.
- Analyse von 1000 Fotos dauert auf dem Testrechner in einem für den Nutzer akzeptablen Rahmen (Schwelle mit dem Auftraggeber festlegen).
- Der Druckdienst nimmt mindestens einen unserer Exportwege an, oder der assistierte Weg ist praktikabel.

## Nicht Teil von Etappe 0

Oberfläche, Seitenplan-Optimierung, Installer, Gesichts- und Augenerkennung im Detail.
