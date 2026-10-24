# Handbuch: Fotobuch-Assistent

Der Assistent erledigt die Vorarbeit für dein Fotobuch: Er sortiert nach Aufnahmezeit, teilt in Ereignisse, sortiert Doppelte und fast Gleiche vor und rechnet die Seiten. Danach exportierst du die Fotos in der richtigen Reihenfolge zu deinem Druckdienst.

**Alles läuft auf deinem Rechner.** Es gibt kein Konto, keinen Upload und keine Cloud-KI. Deine Originalfotos werden nie verändert oder gelöscht.

## Starten

1. Doppelklick auf `fotobuch-assistent.exe`. Es öffnet sich ein schwarzes Fenster (nicht schliessen, solange du arbeitest) und dein Browser mit der App.
2. Windows meldet beim ersten Start eventuell «Der Computer wurde durch Windows geschützt», weil die Datei noch nicht signiert ist. Klicke auf «Weitere Informationen» und «Trotzdem ausführen».
3. Öffnet sich der Browser nicht, kopiere die Adresse (beginnt mit `http://127.0.0.1:`) aus dem schwarzen Fenster in die Adresszeile.
4. Beenden: im letzten Schritt «App beenden» oder das schwarze Fenster schliessen. Deine Auswahl ist gespeichert.

## Die fünf Schritte

Oben siehst du die Schritte. Rechts zeigt «Dein Buch» immer, wie viele Fotos und Seiten das Buch gerade hat.

### 1. Fotos laden
Wähle den Ordner mit den Fotos (Unterordner werden mitgelesen) und klicke «Fotos analysieren». Unterstützt sind JPG, HEIC (iPhone), PNG und RAW (über die eingebettete Vorschau). Bei grossen Ordnern dauert das einige Minuten. Beim zweiten Mal geht es schnell, weil nur neue oder geänderte Fotos gelesen werden.

### 2. Ereignisse
Der Assistent teilt die Fotos in Ereignisse. Eine neue Gruppe beginnt nach einer Pause (Standard sechs Stunden) oder bei einem Ortssprung über 50 km. Du kannst die Pause ändern und jedem Ereignis einen Namen geben. Fotos ohne Aufnahmezeit werden gewarnt.

### 3. Aussortieren
- **Exakte Duplikate** (gleiche Datei) und **Screenshots** sind schon aussortiert. Unten in «Aussortiert» holst du sie mit einem Klick zurück.
- **Serien** sind Fotos, die kurz nacheinander fast gleich aufgenommen wurden. Pro Serie schlägt der Assistent das schärfste Foto mit der besten Auflösung vor. Klicke ein Foto an, um es zu behalten oder zu entfernen. Es dürfen mehrere bleiben.
- «Alle Vorschläge übernehmen» setzt alle Serien auf den Vorschlag zurück.

Tastatur in den Serien: Pfeiltasten wählen ein Foto, Leertaste wechselt behalten/entfernen, N und P springen zur nächsten oder vorherigen Serie, V setzt auf den Vorschlag, G zeigt das Foto gross.

### 4. Seiten
Stelle ein, wie viele Fotos durchschnittlich auf eine Seite sollen, und wähle den Druckdienst. Der Zähler zeigt:
- wie viele Seiten die Fotos brauchen und wie viele das Buch haben wird (Druckdienste erlauben nur bestimmte Seitenzahlen),
- wie viele Fotos bis zur vollen Seitenzahl fehlen,
- wie viele Fotos du weglassen müsstest, um eine Stufe zu sparen.

«Auffüllen» und «Kürzen» schlagen konkrete Fotos vor: zum Auffüllen die besten aussortierten Serienreste, zum Kürzen die schwächsten Fotos genau dort, wo eine Seite wegfällt. Fotos, die du selbst behalten hast, schlägt der Assistent nie zum Entfernen vor.

Die Option «Jedes Ereignis beginnt auf einer neuen Seite» macht das Buch übersichtlicher, braucht aber oft mehr Seiten.

**Wichtig:** Die Seitenregeln der Anbieter (Mindestseitenzahl, Schrittweite, Höchstzahl) sind noch nicht geprüft. Gelb markierte Profile zeigen das. Prüfe die Werte bei deinem Druckdienst und passe sie unter «Seitenregeln anpassen» an.

### 5. Export
Der Assistent kopiert die Fotos nummeriert in einen Ordner pro Ereignis und schreibt `seitenliste.csv` (welches Foto auf welche Seite) und `LIESMICH.txt`. Lade danach die Fotos in der Software deines Druckdienstes hoch. Ein direkter Import in Cewe, Pixum oder Ifolor ist noch nicht möglich. Ein bestehender, gefüllter Zielordner wird nie überschrieben.

## Wo liegen meine Daten?
Im Fotoordner entsteht der Unterordner `.fotobuch` (Zwischenspeicher, deine Entscheidungen, Vorschaubilder). Du kannst ihn jederzeit löschen. Die Fotos bleiben unberührt, nur deine Entscheidungen gehen dann verloren.

## Wenn etwas nicht klappt
| Problem | Lösung |
|---|---|
| Browser zeigt «Die App antwortet nicht» | Das schwarze Fenster wurde geschlossen. Die .exe neu starten. |
| «In diesem Ordner wurden keine Fotos gefunden» | Anderen Ordner wählen. Dateien in `.fotobuch`, `fotobuch-export` und `fotobuch-bericht` werden bewusst übersprungen. |
| Datei «nicht lesbar» | Die Datei ist beschädigt oder ein unbekanntes Format. Sie kommt nicht ins Buch. |
| Reihenfolge stimmt nicht | Das Foto hat keine Aufnahmezeit und wurde nach Änderungsdatum eingeordnet. |
| Der Ordner-Dialog fehlt | Pfad von Hand in das Feld eingeben. |

## Fehler melden
Zum Teilen mit dem Entwickler dient der Bericht ohne Bilder: `fotobuch-assistent.exe bericht` erzeugt `bericht-teilbar.json` (keine Bilder, Namen, Pfade oder GPS-Daten).
