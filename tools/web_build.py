"""Baut die zweisprachige Webseite (website/de, website/en) aus Vorlage und Texten.
Aufruf: python tools/web_build.py <ordner-mit-screenshots>   (Ergebnis von tools/web_screens.py)"""
import shutil
import sys
from pathlib import Path

from PIL import Image

WEB = Path(__file__).resolve().parent.parent / "website"
REPO = "https://github.com/goerks/fotobuch-assistent"
DL = REPO + "/releases/latest/download/"
THEMEN = ["klar", "leinen", "nacht", "kraft", "meer", "galerie"]

BILDER = {  # Quelle -> (Zielname, max. Breite)
    "1-start": ("start", 1360), "2-ereignisse": ("ereignisse", 1360), "3-aussortieren": ("aussortieren", 1360), "3b-orte": ("orte", 1360),
    "4-seiten": ("seiten", 1360), "5-gestaltung": ("gestaltung", 1360), "6-buch": ("buch", 1360), "6b-layouts": ("layouts", 1360),
    "6c-panorama": ("panorama", 1360), "6d-foto-detail": ("detail", 1360), "6e-uebersicht": ("uebersicht", 1360), "6f-seitenband": ("seitenband", 1360), "7-export": ("export", 1360),
}


def bilder(quelle: Path):
    ziel = WEB / "img"
    shutil.rmtree(ziel, ignore_errors=True); ziel.mkdir(parents=True)
    for q, (n, w) in BILDER.items():
        im = Image.open(quelle / f"{q}.png").convert("RGB")
        if im.width > w:
            im = im.resize((w, round(im.height * w / im.width)), Image.Resampling.LANCZOS)
        im.save(ziel / f"{n}.webp", quality=84, method=6)
    for p in quelle.glob("buch-*.jpg"):
        Image.open(p).save(ziel / (p.stem + ".webp"), quality=82, method=6)
    # Titelbild: fertiges Deckblatt aus drei Themen
    print(len(list(ziel.iterdir())), "Bilder", sum(f.stat().st_size for f in ziel.iterdir()) // 1024, "KB")


T = {
 "de": dict(
  lang="de", titel="Fotobuch-Assistent – aus tausend Fotos wird dein Fotobuch", beschr="Der Fotobuch-Assistent ordnet deine Fotos offline: Ereignisse, Duplikate, Seiten, Layouts und fertige Druckseiten. Für Windows, Mac und Linux.",
  nav=[("funktionen", "Funktionen"), ("ablauf", "So geht’s"), ("themen", "Gestaltung"), ("download", "Download"), ("faq", "Fragen")],
  andere="English", andere_url="../en/",
  h1="Aus tausend Fotos wird dein Fotobuch.", sub="Der Fotobuch-Assistent sortiert, gruppiert und plant deine Ferienfotos, ganz auf deinem eigenen Rechner. Du entscheidest nur noch, was dir wichtig ist.",
  cta="Herunterladen", cta2="So funktioniert’s", fuer="Für {os}", alle="Alle Versionen",
  stripe=["100 % offline", "Originale bleiben unberührt", "Kein Konto, kein Upload", "Windows · Mac · Linux", "Quellcode öffentlich"],
  f_h="Was die App kann", f_s="Die Vorarbeit für ein Fotobuch ist mühsam. Der Assistent übernimmt sie und lässt dir die Auswahl.",
  feat=[("🗓", "Ereignisse nach Zeit und Ort", "Aus Aufnahmezeit und GPS bildet die App Tage, Ausflüge und Anlässe. Du benennst, teilst oder legst sie zusammen."),
        ("🧹", "Doppelte und Serien erkennen", "Exakte Kopien und fast gleiche Fotos werden zusammengefasst. Vorgeschlagen wird das schärfste, beste Bild."),
        ("📱", "Screenshots und Chat-Kopien raus", "Bildschirmfotos und Messenger-Kopien landen gar nicht erst im Buch. Du kannst jede Entscheidung ändern."),
        ("🔢", "Der Seitenzähler rechnet mit", "Du gibst einen Bereich vor, zum Beispiel 2 bis 5 Fotos pro Seite. Kleine Ereignisse bekommen eine Seite, grosse mehrere, wenn möglich eine Doppelseite. Der Zähler zeigt live die Seitenzahl."),
        ("🧩", "Layouts pro Seite", "Hoch- und Querformate werden passend gemischt. Für jede Seite wählst du eine Vorlage, auch randlos über die ganze Seite."),
        ("🌄", "Panoramen über die Doppelseite", "Sehr breite Fotos laufen über beide Seiten oder bekommen eine eigene. Die Seitenzahl bleibt dabei stimmig."),
        ("🎨", "Sechs Gestaltungsthemen", "Deckblatt, Hintergrund, Rahmen und Schrift passen zusammen. Titel und Untertitel ergänzt die App aus Datum und Ereignis."),
        ("🖨", "Fertige Seiten für den Druck", "Export als JPEG und PDF in 300 dpi, dazu eine Seitenliste. Fotos von iPhone (HEIC) und Kamera werden gelesen.")],
  a_h="Schritt für Schritt zum Album", a_s="Sieben geführte Schritte. Du kannst jederzeit zurück, und alles, was du entschieden hast, bleibt gespeichert.",
  schritte=[("start", "Fotos laden", "Ordner wählen, fertig. Die App liest Aufnahmezeit, Ort und Format direkt aus den Dateien. Deine Originale werden nur gelesen, nie verändert."),
            ("ereignisse", "Ereignisse prüfen", "Die App teilt die Reise nach Pausen und Orten in Ereignisse. Mit dem Regler stellst du die Feinheit ein, auf der Zeitleiste teilst und verbindest du Ereignisse. Jedes Ereignis zeigt, welche Fotos ins Buch kommen."),
            ("aussortieren", "Aussortieren", "Serien und Doppelte sind schon zusammengefasst, das beste Foto ist vorgewählt. Das Serienband zeigt alle Serien mit Fortschritt, Balken für Schärfe und Auflösung begründen den Vorschlag."),
            ("seiten", "Umfang festlegen", "Bereich pro Seite und Dichte je Ereignis wählen oder eine Seitenzahl vorgeben. Die Galerie je Ereignis zeigt, welche Fotos es ins Buch geschafft haben, und lässt dich ändern."),
            ("gestaltung", "Gestalten", "Ein Thema wählen, Titel eintragen, Titelfoto bestimmen. Die Vorschau zeigt Deckblatt und die ersten Seiten sofort."),
            ("buch", "Buch durchblättern", "Doppelseite für Doppelseite wie im gedruckten Buch. Das Seitenband zeigt alle Seiten: verschieben, leere Seiten einfügen, Seiten oder ganze Ereignisse umstellen."),
            ("export", "Exportieren", "Ein Klick erzeugt fertige Seiten als JPEG und PDF plus Seitenliste. Fotos hochladen oder Seiten direkt weitergeben.")],
  extra_h="Im Buch: volle Kontrolle", extra=[("layouts", "Layout wählen", "Pro Seite eine Vorlage, von drei Fotos bis randlos."), ("panorama", "Panorama", "Breite Bilder über die ganze Doppelseite."), ("detail", "Foto im Detail", "Gross ansehen, verschieben oder entfernen."), ("uebersicht", "Übersicht", "Alle Seiten auf einen Blick."), ("seitenband", "Seitenband", "Seiten ziehen, leere Seiten einfügen, Ereignisse umstellen.")],
  t_h="Sechs Themen, ein Klick", t_s="Das sind echte Seiten aus der App, mit dem Demo-Album «Sommer am Lago Maggiore». Deckblatt und zwei Innenseiten.",
  themen={"klar": "Klar", "leinen": "Leinen", "nacht": "Nacht", "kraft": "Kraft", "meer": "Meer", "galerie": "Galerie"},
  d_h="Herunterladen", d_s="Kostenlos, portabel und ohne Installation. Entpacken oder starten, fertig.",
  dl=[("windows", "Windows", "Windows 10 und 11, 64 Bit", "Fotobuch-Assistent-windows.exe", "Herunterladen (.exe)", ["Datei herunterladen und doppelklicken.", "Meldet Windows «Der Computer wurde geschützt», klicke «Weitere Informationen» und «Trotzdem ausführen». Die App ist noch nicht signiert."]),
      ("mac", "Mac", "macOS, Apple Silicon (M1 und neuer)", "Fotobuch-Assistent-mac.zip", "Herunterladen (.zip)", ["ZIP öffnen, die App in den Programme-Ordner ziehen.", "Erster Start: Rechtsklick, «Öffnen», bestätigen. Die App ist noch nicht von Apple signiert."]),
      ("linux", "Linux", "64 Bit (x86_64), WebKitGTK 4.1", "Fotobuch-Assistent-linux-x86_64.tar.gz", "Herunterladen (.tar.gz)", ["Archiv entpacken und «fotobuch-assistent» starten.", "Benötigt WebKitGTK 4.1, auf den meisten Desktops vorhanden (Debian/Ubuntu: libwebkit2gtk-4.1-0)."])],
  alle_dl="Alle Versionen und Änderungen auf GitHub",
  ehrlich_h="Ehrlich gesagt", ehrlich=["Die App bereitet das Buch vor. Ein direkter Import in die Software von Cewe, Pixum oder Ifolor ist noch nicht möglich, du lädst die fertigen Seiten dort hoch.", "Die Seitenregeln der Druckdienste (Mindestseitenzahl, Schrittweite) sind Annahmen und lassen sich anpassen. Prüfe sie bei deinem Anbieter.", "Die Oberfläche ist auf Deutsch. Eine englische Oberfläche ist noch nicht vorhanden."],
  q_h="Häufige Fragen",
  faq=[("Werden meine Fotos hochgeladen?", "Nein. Die App arbeitet komplett auf deinem Rechner und braucht keine Internetverbindung. Es gibt kein Konto und keine Cloud."),
       ("Verändert die App meine Originale?", "Nein. Fotos werden nur gelesen. Deine Entscheidungen liegen in einem Unterordner «.fotobuch», den du jederzeit löschen kannst."),
       ("Läuft das auch auf einem älteren PC?", "Ja, die App ist auf bescheidene Rechner ausgelegt. Vorschaubilder werden einmal berechnet und dann zwischengespeichert."),
       ("Welche Fotoformate werden gelesen?", "JPEG, PNG und HEIC vom iPhone. Aufnahmezeit und GPS liest die App, wenn sie in der Datei stehen. Fotos ohne Zeit stehen als eigenes Ereignis am Schluss."),
       ("Was kostet das?", "Das Herunterladen und Verwenden kostet nichts."),
       ("Kann ich das Buch direkt bestellen?", "Noch nicht. Du exportierst fertige Seiten oder die nummerierten Fotos mit Seitenliste und lädst sie bei deinem Druckdienst hoch.")],
  fuss="Der Fotobuch-Assistent arbeitet offline. Die Bilder dieser Seite zeigen die echte App mit einem gezeichneten Demo-Set, keine privaten Fotos.", handbuch="Handbuch", code="Quellcode", ver="Version 0.1"),
 "en": dict(
  lang="en", titel="Photo Book Assistant – turn a thousand photos into your photo book", beschr="The Photo Book Assistant sorts your photos offline: events, duplicates, pages, layouts and print-ready pages. For Windows, Mac and Linux.",
  nav=[("funktionen", "Features"), ("ablauf", "How it works"), ("themen", "Design"), ("download", "Download"), ("faq", "FAQ")],
  andere="Deutsch", andere_url="../de/",
  h1="From a thousand photos to your photo book.", sub="The Photo Book Assistant sorts, groups and plans your holiday photos, entirely on your own computer. You only decide what matters to you.",
  cta="Download", cta2="See how it works", fuer="For {os}", alle="All versions",
  stripe=["100% offline", "Originals stay untouched", "No account, no upload", "Windows · Mac · Linux", "Source code public"],
  f_h="What the app does", f_s="Preparing a photo book is tedious. The assistant does the groundwork and leaves the choices to you.",
  feat=[("🗓", "Events by time and place", "From capture time and GPS the app forms days, outings and occasions. You rename, split or merge them."),
        ("🧹", "Spots duplicates and bursts", "Exact copies and near-identical shots are grouped. The sharpest, best picture is suggested."),
        ("📱", "Screenshots and chat copies out", "Screenshots and messenger copies never reach the book. Every decision can be changed."),
        ("🔢", "A page counter that does the math", "You set a range, for example 2 to 5 photos per page. Small events get one page, big ones several, ideally a spread. The counter shows the page count live."),
        ("🧩", "Layouts per page", "Portrait and landscape photos are mixed to fit. Pick a template per page, including full-bleed."),
        ("🌄", "Panoramas across the spread", "Very wide photos span both pages or get a page of their own. The page count stays consistent."),
        ("🎨", "Six design themes", "Cover, background, frames and typeface match. Title and subtitle are filled in from date and event."),
        ("🖨", "Print-ready pages", "Export as JPEG and PDF at 300 dpi, plus a page list. Reads photos from iPhone (HEIC) and cameras.")],
  a_h="Step by step to your album", a_s="Seven guided steps. Go back any time; everything you decided is saved.",
  schritte=[("start", "Load photos", "Pick a folder, done. The app reads capture time, place and format straight from the files. Your originals are only read, never changed."),
            ("ereignisse", "Check events", "The app splits the trip into events by gaps and places. A slider sets the granularity and the timeline lets you split and merge events. Each event shows which photos make it into the book."),
            ("aussortieren", "Cull", "Bursts and duplicates are already grouped and the best photo is preselected. A burst strip shows all series with progress, and bars for sharpness and resolution explain the suggestion."),
            ("seiten", "Set the scope", "Choose a range per page and a density per event, or set a page count. A gallery per event shows which photos made it into the book and lets you override."),
            ("gestaltung", "Design", "Choose a theme, enter a title, pick the cover photo. The preview shows the cover and first pages right away."),
            ("buch", "Flip through the book", "Spread by spread, like the printed book. The page strip shows every page: move them, insert blank pages, reorder pages or whole events."),
            ("export", "Export", "One click makes finished pages as JPEG and PDF plus a page list. Upload the photos or hand over the pages.")],
  extra_h="In the book: full control", extra=[("layouts", "Choose a layout", "One template per page, from three photos to full-bleed."), ("panorama", "Panorama", "Wide pictures across the whole spread."), ("detail", "Photo detail", "View large, move or remove."), ("uebersicht", "Overview", "All pages at a glance."), ("seitenband", "Page strip", "Drag pages, insert blank ones, reorder events.")],
  t_h="Six themes, one click", t_s="These are real pages from the app, using the demo album “Summer at Lake Maggiore”. Cover and two inner pages.",
  themen={"klar": "Clear", "leinen": "Linen", "nacht": "Night", "kraft": "Kraft", "meer": "Sea", "galerie": "Gallery"},
  d_h="Download", d_s="Free, portable, no installation. Unpack or launch, done.",
  dl=[("windows", "Windows", "Windows 10 and 11, 64-bit", "Fotobuch-Assistent-windows.exe", "Download (.exe)", ["Download the file and double-click it.", "If Windows shows “protected your PC”, click “More info” and “Run anyway”. The app is not signed yet."]),
      ("mac", "Mac", "macOS, Apple silicon (M1 and newer)", "Fotobuch-Assistent-mac.zip", "Download (.zip)", ["Open the ZIP and drag the app to Applications.", "First launch: right-click, “Open”, confirm. The app is not signed by Apple yet."]),
      ("linux", "Linux", "64-bit (x86_64), WebKitGTK 4.1", "Fotobuch-Assistent-linux-x86_64.tar.gz", "Download (.tar.gz)", ["Unpack the archive and run “fotobuch-assistent”.", "Needs WebKitGTK 4.1, present on most desktops (Debian/Ubuntu: libwebkit2gtk-4.1-0)."])],
  alle_dl="All versions and changes on GitHub",
  ehrlich_h="To be honest", ehrlich=["The app prepares the book. Importing directly into Cewe, Pixum or Ifolor software is not possible yet; you upload the finished pages there.", "Print-service page rules (minimum pages, step size) are assumptions and can be adjusted. Check them with your provider.", "The interface is in German. There is no English interface yet."],
  q_h="Frequently asked questions",
  faq=[("Are my photos uploaded?", "No. The app works entirely on your computer and needs no internet connection. There is no account and no cloud."),
       ("Does the app change my originals?", "No. Photos are only read. Your decisions live in a “.fotobuch” subfolder that you can delete at any time."),
       ("Does it run on an older PC?", "Yes, the app is built for modest computers. Thumbnails are computed once and then cached."),
       ("Which photo formats are read?", "JPEG, PNG and iPhone HEIC. Capture time and GPS are read when stored in the file. Photos without a time appear as their own event at the end."),
       ("What does it cost?", "Downloading and using it is free."),
       ("Can I order the book directly?", "Not yet. You export finished pages, or numbered photos with a page list, and upload them to your print service.")],
  fuss="The Photo Book Assistant works offline. Images on this page show the real app with a drawn demo set, not private photos.", handbuch="Manual (German)", code="Source code", ver="Version 0.1"),
}


def seite(sp: str, t: dict) -> str:
    nav = "".join(f'<a href="#{a}">{b}</a>' for a, b in t["nav"])
    stripe = "".join(f"<li>{x}</li>" for x in t["stripe"])
    feat = "".join(f'<article class="karte"><div class="ico" aria-hidden="true">{i}</div><h3>{h}</h3><p>{p}</p></article>' for i, h, p in t["feat"])
    schritte = "".join(f'''<li class="schritt"><div class="text"><span class="nr">{i + 1}</span><h3>{h}</h3><p>{p}</p></div>
      <figure class="rahmen"><img src="../img/{b}.webp" alt="{h}" loading="lazy" width="1360" height="860"></figure></li>''' for i, (b, h, p) in enumerate(t["schritte"]))
    extra = "".join(f'<figure class="klein"><div class="rahmen"><img src="../img/{b}.webp" alt="{h}" loading="lazy"></div><figcaption><b>{h}</b> {p}</figcaption></figure>' for b, h, p in t["extra"])
    tabs = "".join(f'<button role="tab" data-t="{k}" aria-selected="{"true" if i == 1 else "false"}">{t["themen"][k]}</button>' for i, k in enumerate(THEMEN))
    thema_bilder = "".join(
        f'<div class="themapanel" data-t="{k}"{"" if i == 1 else " hidden"}>' + "".join(
            f'<img src="../img/buch-{k}-{n}.webp" alt="{t["themen"][k]}" loading="lazy" width="1000" height="750">' for n in ("deckblatt", "seite-a", "seite-b")) + "</div>"
        for i, k in enumerate(THEMEN))
    dl = "".join(f'''<article class="dlkarte" id="dl-{k}"><h3>{n}</h3><p class="sys">{sys_}</p>
      <a class="knopf" href="{DL}{datei}" download>{txt}</a><ul>{"".join(f"<li>{x}</li>" for x in tipps)}</ul></article>''' for k, n, sys_, datei, txt, tipps in t["dl"])
    ehrlich = "".join(f"<li>{x}</li>" for x in t["ehrlich"])
    faq = "".join(f"<details><summary>{q}</summary><p>{a}</p></details>" for q, a in t["faq"])
    texte = {"windows": "Windows", "mac": "Mac", "linux": "Linux"}
    return f'''<!doctype html>
<html lang="{t["lang"]}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{t["titel"]}</title><meta name="description" content="{t["beschr"]}">
<link rel="alternate" hreflang="de" href="../de/"><link rel="alternate" hreflang="en" href="../en/">
<meta property="og:title" content="{t["titel"]}"><meta property="og:description" content="{t["beschr"]}"><meta property="og:image" content="../img/buch-leinen-deckblatt.webp">
<link rel="icon" href="../img/icon.png"><link rel="stylesheet" href="../site.css"></head>
<body>
<header class="kopf"><div class="breit kopfinnen"><a class="logo" href="#"><img src="../img/icon.png" alt="" width="28" height="28"> Fotobuch-Assistent</a>
<nav aria-label="Navigation">{nav}</nav><a class="sprache" href="{t["andere_url"]}" hreflang="{"en" if t["lang"] == "de" else "de"}">{t["andere"]}</a></div></header>
<main>
<section class="held"><div class="breit heldinnen"><div class="heldtext"><p class="etikett">{t["ver"]} · Windows · Mac · Linux</p><h1>{t["h1"]}</h1><p class="sub">{t["sub"]}</p>
<div class="knopfzeile"><a class="knopf gross" id="hauptdl" href="#download" data-fuer="{t["fuer"]}">{t["cta"]}</a><a class="knopf hell gross" href="#ablauf">{t["cta2"]}</a></div>
<p class="klein" id="hauptdl-info"><a href="#download">{t["alle"]}</a></p></div>
<div class="heldbild"><div class="rahmen"><img src="../img/buch.webp" alt="" width="1360" height="860" fetchpriority="high"></div>
<img class="schweber s1" src="../img/buch-leinen-deckblatt.webp" alt="" width="1000" height="750"><img class="schweber s2" src="../img/buch-nacht-deckblatt.webp" alt="" width="1000" height="750"></div></div></section>
<section class="streifen"><ul class="breit">{stripe}</ul></section>
<section id="funktionen" class="block"><div class="breit"><h2>{t["f_h"]}</h2><p class="lead">{t["f_s"]}</p><div class="raster">{feat}</div></div></section>
<section id="ablauf" class="block getoent"><div class="breit"><h2>{t["a_h"]}</h2><p class="lead">{t["a_s"]}</p><ol class="schritte">{schritte}</ol>
<h3 class="zwischen">{t["extra_h"]}</h3><div class="raster4">{extra}</div></div></section>
<section id="themen" class="block"><div class="breit"><h2>{t["t_h"]}</h2><p class="lead">{t["t_s"]}</p><div class="tabs" role="tablist">{tabs}</div><div class="themabuehne">{thema_bilder}</div></div></section>
<section id="download" class="block getoent"><div class="breit"><h2>{t["d_h"]}</h2><p class="lead">{t["d_s"]}</p><div class="dlraster">{dl}</div>
<p class="mitte"><a href="{REPO}/releases">{t["alle_dl"]}</a></p>
<div class="ehrlich"><h3>{t["ehrlich_h"]}</h3><ul>{ehrlich}</ul></div></div></section>
<section id="faq" class="block"><div class="schmal"><h2>{t["q_h"]}</h2>{faq}</div></section>
</main>
<footer class="fuss"><div class="breit"><p>{t["fuss"]}</p><p><a href="{REPO}">{t["code"]}</a> · <a href="{REPO}/blob/main/docs/handbuch.md">{t["handbuch"]}</a> · <a href="{REPO}/releases">Releases</a></p></div></footer>
<script src="../site.js"></script></body></html>'''


def main(quelle: Path):
    bilder(quelle)
    icon = WEB.parent / "crates" / "app" / "icons" / "128x128.png"
    shutil.copy(icon, WEB / "img" / "icon.png")
    for sp in ("de", "en"):
        (WEB / sp).mkdir(exist_ok=True)
        (WEB / sp / "index.html").write_text(seite(sp, T[sp]), encoding="utf-8")
    (WEB / ".nojekyll").write_text("")
    print("Seiten geschrieben")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
