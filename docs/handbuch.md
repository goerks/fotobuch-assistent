# Handbuch: Fotobuch-Assistent

Der Assistent erledigt die Vorarbeit für dein Fotobuch: Er sortiert nach Aufnahmezeit, teilt in Ereignisse, sortiert Doppelte und fast Gleiche vor und rechnet die Seiten. Danach exportierst du die Fotos in der richtigen Reihenfolge zu deinem Druckdienst.

**Alles läuft auf deinem Rechner.** Es gibt kein Konto, keinen Upload und keine Cloud-KI. Deine Originalfotos werden nie verändert oder gelöscht.

## Starten

1. Doppelklick auf `fotobuch-assistent.exe`. Es öffnet sich ein eigenes Fenster, kein Browser und keine Konsole. Die App braucht keine Installation. Voraussetzung ist Microsoft Edge WebView2, das auf Windows 10 und 11 normalerweise schon vorhanden ist.
2. Windows meldet beim ersten Start eventuell «Der Computer wurde durch Windows geschützt», weil die Datei noch nicht signiert ist. Klicke auf «Weitere Informationen» und «Trotzdem ausführen».
3. Beenden: im letzten Schritt «App beenden» oder das Fenster schliessen. Deine Auswahl ist gespeichert.

## Die sechs Schritte

Links siehst du den Ablauf von oben nach unten: Fotos laden, Ereignisse, Aussortieren, Seiten, Buch, Export. Erledigte Schritte sind mit einem Haken markiert, daneben steht je eine Zahl (zum Beispiel Fotos oder Seiten). Rechts zeigt «Dein Buch» immer, wie viele Fotos und Seiten das Buch gerade hat.

### 1. Fotos laden
Wähle den Ordner mit den Fotos (Unterordner werden mitgelesen) und klicke «Fotos analysieren». Unterstützt sind JPG, HEIC (iPhone), PNG und RAW (über die eingebettete Vorschau). Bei grossen Ordnern dauert das einige Minuten. Beim zweiten Mal geht es schnell, weil nur neue oder geänderte Fotos gelesen werden.

### 2. Ereignisse
Der Assistent teilt die Fotos in Ereignisse. Eine neue Gruppe beginnt nach einer Pause (Standard sechs Stunden) oder bei einem Ortssprung über 50 km. Du kannst die Pause ändern und jedem Ereignis einen Namen geben. Fotos ohne Aufnahmezeit (z. B. aus einem Chat gespeichert) bilden ein eigenes Ereignis «Ohne Aufnahmezeit» am Schluss des Buchs, nach Dateiname geordnet.

### 3. Aussortieren
- **Exakte Duplikate** (gleiche Datei) und **Screenshots** sind schon aussortiert. Unten in «Aussortiert» holst du sie mit einem Klick zurück.
- **Serien** sind Fotos, die kurz nacheinander fast gleich aufgenommen wurden. Pro Serie schlägt der Assistent das schärfste Foto mit der besten Auflösung vor. Klicke ein Foto an, um es zu behalten oder zu entfernen. Es dürfen mehrere bleiben.
- «Alle Vorschläge übernehmen» setzt alle Serien auf den Vorschlag zurück.

Tastatur in den Serien: Pfeiltasten wählen ein Foto, Leertaste wechselt behalten/entfernen, N und P springen zur nächsten oder vorherigen Serie, V setzt auf den Vorschlag, G zeigt das Foto gross.

### 4. Umfang
Gib einen Bereich an, wie viele Fotos auf eine Seite sollen (zum Beispiel 2 bis 5), und wähle den Druckdienst. Der Assistent gibt jedem Ereignis so viele Seiten, dass seine Fotos in den Bereich passen: ein kleines Ereignis eine Seite, ein grosses mehrere, auch eine ungerade Zahl. Doppelseiten werden bevorzugt, die meisten Ereignisse beginnen deshalb auf einer linken Seite. Ist das nicht sinnvoll möglich, beginnt ein Ereignis rechts.

- Pro Ereignis stellst du die Dichte (kompakt, normal, gross) ein oder legst die Seitenzahl mit Plus und Minus selbst fest. «Automatisch» nimmt die eigene Seitenzahl zurück.
- «Buchgrösse in Seiten» legt ein Ziel fest. Leer lassen heisst: so viele Seiten wie nötig. Der Zähler zeigt die Seitenzahl, die der Druckdienst erlaubt, und bietet die nächstkleinere oder nächstgrössere Stufe an, wenn sie sich mit dem Bereich erreichen lässt.
- Unter jedem Ereignis zeigt die Auswahl-Galerie, welche Fotos im Buch sind (mit Seitenzahl) und welche aussortiert wurden (mit Grund). Ein Klick holt ein Foto ins Buch oder nimmt es heraus. Solche eigenen Entscheidungen sind blau markiert und bleiben erhalten.

Die Option «Jedes Ereignis beginnt auf einer neuen Seite» macht das Buch übersichtlicher, braucht aber oft mehr Seiten.

**Wichtig:** Die Seitenregeln der Anbieter (Mindestseitenzahl, Schrittweite, Höchstzahl) sind noch nicht geprüft. Gelb markierte Profile zeigen das. Prüfe die Werte bei deinem Druckdienst und passe sie unter «Seitenregeln anpassen» an.

### 5. Export
Der Assistent kopiert die Fotos nummeriert in einen Ordner pro Ereignis und schreibt `seitenliste.csv` (welches Foto auf welche Seite) und `LIESMICH.txt`. Lade danach die Fotos in der Software deines Druckdienstes hoch. Ein direkter Import in Cewe, Pixum oder Ifolor ist noch nicht möglich. Ein bestehender, gefüllter Zielordner wird nie überschrieben.

## Wo liegen meine Daten?
Im Fotoordner entsteht der Unterordner `.fotobuch` (Zwischenspeicher, deine Entscheidungen, Vorschaubilder). Du kannst ihn jederzeit löschen. Die Fotos bleiben unberührt, nur deine Entscheidungen gehen dann verloren.

## Wenn etwas nicht klappt
| Problem | Lösung |
|---|---|
| «In diesem Ordner wurden keine Fotos gefunden» | Anderen Ordner wählen. Dateien in `.fotobuch`, `fotobuch-export` und `fotobuch-bericht` werden bewusst übersprungen. |
| Datei «nicht lesbar» | Die Datei ist beschädigt oder ein unbekanntes Format. Sie kommt nicht ins Buch. |
| Reihenfolge stimmt nicht | Das Foto hat keine Aufnahmezeit und steht im Ereignis «Ohne Aufnahmezeit» am Schluss. |
| Der Ordner-Dialog fehlt | Pfad von Hand in das Feld eingeben. |

## Mac (Apple Silicon)
Die Mac-Version ist ein Programm für M1 und neuer (`Fotobuch-Assistent-mac.zip`). Doppelklick entpackt die Datei zu `Fotobuch-Assistent.app`. Du kannst die App an einen beliebigen Ort legen, zum Beispiel ins Programme-Verzeichnis. Eine Installation ist nicht nötig. Sie ist noch nicht von Apple signiert, deshalb blockiert macOS den ersten Start:

1. Rechtsklick auf die App, «Öffnen», und die Rückfrage mit «Öffnen» bestätigen. Wenn macOS sie trotzdem blockiert: Systemeinstellungen, «Datenschutz & Sicherheit», ganz unten «Trotzdem öffnen».
2. Alternativ im Terminal einmal `xattr -dr com.apple.quarantine Fotobuch-Assistent.app` eingeben.

Beim ersten Zugriff auf Ordner wie «Bilder» fragt macOS nach der Erlaubnis. Erlaube sie.

## Hinweis zu HEIC (iPhone)
HEIC-Fotos liest die App mit einem eigenen, in Rust geschriebenen Decoder. HEIC nutzt das Videoformat HEVC, für das Patente bestehen. Die App gibt die Fotos nirgends weiter und verändert sie nicht, sie zeigt sie nur lokal an.

## Ereignisse anpassen
Im Schritt «Ereignisse» hat jedes Ereignis zwei Knöpfe: «Mit vorherigem zusammenlegen» und «Teilen …». Beim Teilen klickst du das Foto, ab dem ein neues Ereignis beginnen soll. «Automatische Einteilung wiederherstellen» macht alle Änderungen rückgängig.

## Ganze Seiten, Panoramen und Paarung
Sehr breite Fotos (Panoramen) bekommen automatisch eine eigene Seite, extrem breite laufen über die ganze Doppelseite. Bei jedem Foto öffnest du die Detailansicht (Klick im Buch) und wählst unter «Anordnung»: mit anderen Fotos, ganze Seite oder über die ganze Doppelseite.

## Fotos auf eine andere Seite legen
In der Detailansicht eines Fotos im Buch: «auf vorherige Seite», «auf nächste Seite» oder eine Seitennummer eingeben und «Verschieben». «An den Ursprung zurück» holt das Foto an seinen Platz. Unter dem Buch setzt «Alle Verschiebungen zurücksetzen» alles zurück. «Layout wählen» bietet für Einzelseiten zusätzlich «Randlos»: Das Foto füllt die ganze Seite und wird dabei am Rand beschnitten. Hochformate und Querformate werden an den Seitengrenzen so gemischt, dass die Seiten gut gefüllt sind.

## Fertige Seiten
Im Schritt «Export» erzeugt «Fertige Seiten erzeugen» für jede Seite ein Bild (JPEG, 300 dpi) und eine PDF mit allen Seiten, im Unterordner «seiten» des Zielordners. Das dauert bei einem grossen Buch einige Minuten, ein Balken zeigt den Fortschritt.

## Gestaltung

Im Schritt «Gestaltung» wählst du ein Thema für das ganze Buch: Klar, Leinen, Nacht, Kraft, Meer oder Galerie. Ein Thema legt Farben, Seitenhintergrund, Fotorahmen, Schrift und Deckblatt fest. Darunter kannst du den Farbton, den Seitenhintergrund und den Fotorahmen ändern, Unterschrift und Seitenzahl ein- oder ausschalten und das Deckblatt beschriften (Titel, Untertitel, Name oder Widmung). Ohne eigene Eingabe nimmt der Assistent den Ordnernamen als Titel und den Zeitraum der Aufnahmen als Untertitel. Das Titelfoto wählst du aus einer Reihe von Fotos aus dem Buch. «Alles auf das Thema zurücksetzen» entfernt deine Änderungen an Farbton, Hintergrund und Rahmen.
Die Vorschau im Programm ist eine Annäherung. Beim Erzeugen der fertigen Seiten (Export) werden Deckblatt und Seiten in Druckqualität gezeichnet. Das Deckblatt liegt als eigene Datei («deckblatt.jpg» und «deckblatt.pdf») neben den Seiten.
