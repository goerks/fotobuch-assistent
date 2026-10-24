# Anbieter-Profile

Pro Druckdienst eine kleine, dokumentierte Datei (geplant: JSON oder YAML). Das Profil steuert Export und Seitenzähler.

Geplante Felder (Entwurf, wird in Etappe 0 mit echten Anbietern geprüft):

| Feld | Bedeutung |
|---|---|
| `name` | Anzeigename des Anbieters |
| `formate` | Buchformate mit Breite und Höhe in mm |
| `seiten_min` | Mindestseitenzahl |
| `seiten_schritt` | Schrittweite der Seitenzahl (z. B. 2 oder 4) |
| `seiten_max` | Höchstseitenzahl |
| `beschnitt_mm` | Beschnitt rundum |
| `einzel_oder_doppelseiten` | Wie Seiten angeliefert werden |
| `dateityp` | z. B. `jpeg`, `pdf`, `pdf-x4` |
| `aufloesung_dpi` | Ziel-Auflösung (meist 300) |
| `farbraum` | z. B. `sRGB` |
| `umschlag_getrennt` | Cover als eigene Datei |
| `dateinamen` | Schema für Seitendateien |
| `quelle` | Link zur Anbieter-Dokumentation |
| `geprueft_am` | Datum der letzten Prüfung |

Ein Profil wird erst als **verifiziert** markiert, wenn ein echter Testexport beim Anbieter angenommen wurde.
