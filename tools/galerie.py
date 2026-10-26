"""Baut aus dem Ordner `galerie/` des Ende-zu-Ende-Laufs eine Markdown-Galerie im Repository.
Aufruf: python tools/galerie.py <e2e-ausgabe/galerie> <runde-name> "<Titel>"
Ergebnis: docs/galerie/<runde-name>.md mit Bildern in docs/galerie/<runde-name>/ (lesbar direkt auf GitHub, auch am Handy)."""
import sys
from pathlib import Path
from PIL import Image

TEXTE = {
    "start": ("Start", "Ordner öffnen und Analyse starten. Alles bleibt lokal."),
    "ereignisse": ("Ereignisse", "Ereignisse aus der Aufnahmezeit mit Namen, Zeitraum und Orten. Fotos ohne Aufnahmezeit stehen am Schluss."),
    "ereignis-teilen": ("Ereignis teilen", "Ein Klick auf ein Foto lässt dort ein neues Ereignis beginnen. Zusammenlegen und Zurücksetzen gehören dazu."),
    "aussortieren": ("Aussortieren", "Serien und Duplikate mit Vorschlag und Begründung."),
    "orte": ("Orte", "GPS-Orte mit Begrenzung pro Ort."),
    "seiten": ("Seiten", "Seitenzähler mit Anbieter-Profil, Auffüllen und Kürzen."),
    "gestaltung": ("Gestaltung: Themen", "Neu: Sechs Themen (Klar, Leinen, Nacht, Kraft, Meer, Galerie) mit passendem Deckblatt. Farbton, Seitenhintergrund und Fotorahmen lassen sich anpassen, dazu Titel, Zeitraum, Name und Titelfoto."),
    "gestaltung-leinen": ("Thema Leinen mit eigenem Titel", "Vorschau mit Deckblatt und den ersten Seiten: weisser Fotorand, Unterschrift in Kursiv, Titelfoto im Leinenfenster."),
    "thema-klar": ("Export im Thema Klar", "Deckblatt, Seite 2 und Seite 3 aus dem echten Export (300 dpi, hier verkleinert)."),
    "thema-leinen": ("Export im Thema Leinen", "Stoffeinband mit Fenster, helle Seiten, weisser Rand um die Fotos."),
    "thema-nacht": ("Export im Thema Nacht", "Schwarze Seiten, Titelfoto ganzflächig, Beschriftung in schmaler Grossschrift."),
    "thema-kraft": ("Export im Thema Kraft", "Kraftpapier, Fotos wie Sofortbilder leicht schief aufgeklebt, Stempel auf dem Deckblatt."),
    "thema-meer": ("Export im Thema Meer", "Verlauf auf dem Deckblatt, Welle am Seitenrand in der Farbe des Themas."),
    "thema-galerie": ("Export im Thema Galerie", "Breiter Rand, dünner Strich, Schild mit Nummer, roter Punkt als Akzent."),
    "foto-detail": ("Foto im Detail", "Begründung, ob und warum ein Foto im Buch ist; Foto als ganze Seite setzen."),
    "panorama-ganze-seite": ("Panorama auf eigener Seite", "Sehr breite Fotos bekommen automatisch eine ganze Seite."),
    "panorama-ueber-doppelseite": ("Panorama über die Doppelseite", "Neu: Sehr breite Fotos laufen über beide Seiten. Links Seite 40, rechts Seite 41, ohne Bruch in der Mitte."),
    "foto-verschieben": ("Foto auf eine andere Seite legen", "Neu: In der Detailansicht lässt sich ein Foto auf die vorherige oder nächste Seite oder auf eine gewählte Seitennummer legen. Dort steht auch die Seitenart (mit anderen Fotos, ganze Seite, über die Doppelseite)."),
    "foto-verschoben": ("Nach dem Verschieben", "Die Seite davor hat ein Foto weniger, die Seite danach eines mehr. «Alle Verschiebungen zurücksetzen» erscheint unter dem Buch."),
    "layout-vorlagen": ("Layout-Vorlagen", "Pro Seite lassen sich Anordnungen wie Vorlagen wählen."),
    "buch-uebersicht": ("Buch, Übersicht", "Alle Seiten auf einen Blick."),
    "buch-doppelseite": ("Buch, Doppelseite", "Blättern durch das Buch mit automatisch gewählten Layouts."),
    "export-fertige-seiten": ("Export mit fertigen Seiten", "Jede Seite als JPEG (300 dpi) und alle Seiten als eine PDF."),
    "fertige-seite-1": ("Fertige Seite 1", "Gerendert aus dem Export."),
    "fertige-seite-2": ("Fertige Seite 2", "Gerenderte Seite aus dem Export, verkleinert."),
}


def main(quelle: Path, runde: str, titel: str) -> None:
    ziel = Path(__file__).resolve().parent.parent / "docs" / "galerie"
    bilder = ziel / runde
    bilder.mkdir(parents=True, exist_ok=True)
    zeilen = [f"# {titel}", "", "Bilder aus dem automatischen Ende-zu-Ende-Lauf (Demo-Reise, synthetische Fotos). Erzeugt mit `tools/galerie.py`.", ""]
    for datei in sorted(quelle.iterdir()):
        name = datei.stem.split("-", 1)[1] if datei.stem[:2].isdigit() else datei.stem
        if name not in TEXTE:
            continue
        img = Image.open(datei).convert("RGB")
        img = img.resize((1000, round(img.height * 1000 / img.width))) if img.width > 1000 else img
        aus = f"{datei.stem}.jpg"
        img.save(bilder / aus, quality=82)
        t, d = TEXTE[name]
        zeilen += [f"## {t}", "", d, "", f"![{t}]({runde}/{aus})", ""]
    (ziel / f"{runde}.md").write_text("\n".join(zeilen), encoding="utf-8")


if __name__ == "__main__":
    main(Path(sys.argv[1]), sys.argv[2], sys.argv[3])
