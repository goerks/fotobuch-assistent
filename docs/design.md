# Oberfläche: drei Richtungen zur Auswahl

Alle drei Vorschläge zeigen dieselben Beispieldaten (296 Fotos, 5 Ereignisse, 184 im Buch, 62 Seiten bei 3 Fotos pro Seite, es fehlen 2 Fotos). Sie liegen als Zeichenfläche in Claude vor. Es sind Entwürfe mit Platzhalter-Bildern, keine echten Fotos und noch kein Code.

| | A Zeitleiste | B Buch zuerst | C Assistent |
|---|---|---|---|
| Idee | Ereignisse links, Fotos in der Mitte, Seitenplan rechts | Die Doppelseite steht im Zentrum, Fotos werden direkt im Buch verschoben | Geführte Schritte: Import, Ereignisse, Aussortieren, Seitenplan, Export |
| Stärke | Überblick über alles, gut für viele Fotos | Das Ergebnis ist sofort sichtbar | Einfachste Bedienung, wenig Entscheidungen auf einmal |
| Schwäche | Dichte Oberfläche | Aussortieren ist weniger im Blick | Weniger Überblick, mehr Klicks |
| Look | Hell, kühl, Blau | Dunkel, Amber | Hell, rund, Grün |

Gemeinsame Regeln, unabhängig von der Wahl:
- Vorschläge sind klar von Entscheidungen getrennt und immer umkehrbar.
- Status wird nie nur über Farbe gezeigt, sondern auch mit Text oder Form (Behalten, Vorschlag, Duplikat, Screenshot).
- Der Seitenzähler ist immer sichtbar (Seiten, Fotos, «es fehlen N»).
- Hinweis «Alles lokal, nichts wird hochgeladen» bleibt sichtbar.

## Entscheidung (24.10.2026)

Gewählt hat der Entwickler im Auftrag des Auftraggebers nach dessen Anforderungen (Durchgängigkeit, Führung, intuitiv): **C Assistent als Gerüst**, ergänzt um **A: ständig sichtbare Ereignisliste und Seitenzähler** (Leiste «Dein Buch»). **B Buch zuerst** wird für MVP 2 zurückgestellt (Seitenplan mit Doppelseiten). Alle drei Entwürfe bleiben als Quelle unter `docs/design/vorschlaege/` erhalten, falls sich die Richtung ändert.
