"""Echter Ende-zu-Ende-Test: Demo-Fotoreise (JPEG mit EXIF/GPS, Serien, Duplikate, Screenshots) wird in einem
echten Chromium durch die ganze App geführt und als Album-Kontaktbogen festgehalten.

Aufruf: pytest tests/e2e -s     (Voraussetzung: cargo build --release, pillow, numpy, playwright + chromium)
Das Album (Bild jeder Doppelseite + Kontaktbogen) landet in $FOTOBUCH_E2E_OUT oder im Temp-Ordner.
"""
import hashlib
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest
from PIL import Image
from playwright.sync_api import sync_playwright

ALLE_THEMEN = ["klar", "leinen", "nacht", "kraft", "meer", "galerie"]
# Zum schnelleren Entwickeln: FOTOBUCH_E2E_THEMEN=klar,nacht
THEMEN = [t for t in os.environ.get("FOTOBUCH_E2E_THEMEN", ",".join(ALLE_THEMEN)).split(",") if t in ALLE_THEMEN]
WURZEL = Path(__file__).resolve().parents[2]
BIN = Path(os.environ.get("FOTOBUCH_BIN", WURZEL / "target" / "release" / ("fotobuch-dev.exe" if os.name == "nt" else "fotobuch-dev")))


def pruefsummen(ordner: Path) -> dict:
    return {str(p.relative_to(ordner)): hashlib.sha1(p.read_bytes()).hexdigest()
            for p in sorted(ordner.rglob("*")) if p.is_file() and ".fotobuch" not in p.parts and "fotobuch-export" not in p.parts}


@pytest.fixture(scope="module")
def umgebung():
    basis = Path(tempfile.mkdtemp(prefix="fb-e2e-"))
    fotos = basis / "New-York-2026"
    r = subprocess.run([sys.executable, str(WURZEL / "tools" / "demo_fotos.py"), str(fotos)], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    ausgabe = Path(os.environ.get("FOTOBUCH_E2E_OUT", basis / "album"))
    ausgabe.mkdir(parents=True, exist_ok=True)
    return basis, fotos, ausgabe


def starte(fotos: Path):
    p = subprocess.Popen([str(BIN), "serve", str(fotos)], stdout=subprocess.PIPE, text=True)
    return p, p.stdout.readline().strip()


def test_reise_wird_zum_album(umgebung):
    basis, fotos, ausgabe = umgebung
    vorher = pruefsummen(fotos)
    anzahl_dateien = len(vorher)
    assert anzahl_dateien > 250

    proc, url = starte(fotos)
    fehler = []
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch()
            seite = browser.new_context(viewport={"width": 1500, "height": 950}, bypass_csp=True).new_page()
            seite.on("pageerror", lambda e: fehler.append(str(e)))
            seite.on("console", lambda m: m.type == "error" and fehler.append(m.text))
            seite.goto(url)
            seite.wait_for_function("window.__fotobuch && window.__fotobuch.S.a")
            galerie = ausgabe / "galerie"
            galerie.mkdir(exist_ok=True)
            nummer = [0]

            def bild(name, voll=False, ziel=None):
                """Screenshot der App für die Galerie (nummeriert in der Reihenfolge des Durchlaufs)."""
                nummer[0] += 1
                seite.wait_for_timeout(250)
                pfad = galerie / f"{nummer[0]:02d}-{name}.png"
                (seite.locator(ziel) if ziel else seite).screenshot(path=str(pfad), full_page=voll) if not ziel else seite.locator(ziel).screenshot(path=str(pfad))

            bild("start")
            # 1. Analyse (Ordner wurde beim Start geöffnet)
            seite.click("#btn-analyse")
            seite.wait_for_function("/Ereignisse/.test(document.querySelector('h1')?.textContent)", timeout=120000)
            a = seite.evaluate("window.__fotobuch.S.a")
            z = a["zahlen"]
            assert z["dateien"] == anzahl_dateien
            assert z["ereignisse"] >= 4, "vier Reisetage"
            assert z["exakte_gruppen"] >= 1, "Messenger-Kopien erkannt"
            assert z["serien"] >= 10
            assert z["screenshots"] >= 1
            assert 0 < z["im_buch"] < z["dateien"]
            assert z["entfernt"] == z["dateien"] - z["im_buch"]
            letztes = a["ereignisse"][-1]
            assert letztes["ohne_datum"] and letztes["anzahl"] >= 5 and letztes["titel"] == "Ohne Aufnahmezeit", "Fotos ohne Zeit am Schluss"
            assert not any(e["ohne_datum"] for e in a["ereignisse"][:-1])
            ereignisse_auto = z["ereignisse"]

            bild("ereignisse", voll=False)
            # 2. Ereignisse benennen, zusammenlegen, teilen, zurücksetzen
            seite.fill('input[data-key] >> nth=0', "Ankunft")
            seite.press('input[data-key] >> nth=0', "Tab")
            seite.wait_for_function("window.__fotobuch.S.a.ereignisse[0].name === 'Ankunft'")
            seite.click('button[data-aktion="zusammenlegen"] >> nth=0')
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.ereignisse === {ereignisse_auto - 1}")
            seite.click('button[data-aktion="teilen"] >> nth=0')
            bild("ereignis-teilen")
            seite.click(".teilen-foto >> nth=10")
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.ereignisse === {ereignisse_auto}")
            seite.click("#ereignisse-zurueck")
            seite.wait_for_function("!window.__fotobuch.S.a.zahlen.ereignis_aenderungen")
            assert seite.evaluate("window.__fotobuch.S.a.zahlen.ereignisse") == ereignisse_auto
            assert seite.evaluate("window.__fotobuch.S.a.ereignisse[0].name") == "Ankunft", "Name bleibt bestehen"

            # Aussortieren, Orte und Seiten für die Galerie
            seite.evaluate("window.__fotobuch.geheZu(3)")
            seite.wait_for_selector("h1")
            bild("aussortieren")
            seite.evaluate("document.querySelector('#orte, .ort')?.scrollIntoView()")
            bild("orte")
            seite.evaluate("window.__fotobuch.geheZu(4)")
            seite.wait_for_selector(".ereigniskarte")
            seite.wait_for_function("[...document.querySelectorAll('.ereigniskarte details[open] img')].filter(i => { const r = i.getBoundingClientRect(); return r.bottom > 0 && r.top < innerHeight; }).every(i => i.complete && i.naturalWidth > 0)", timeout=30000)
            # Umfang: Kein Ereignis wird auf eine feste Zahl gezwungen, kleine haben eine Seite, die Doppelseiten-Wahl bleibt ein Wunsch
            up = seite.evaluate("window.__fotobuch.S.a.seitenplan")
            assert seite.locator(".ereigniskarte").count() == len(up["ereignisse"])
            assert all(e["seiten"] >= 1 for e in up["ereignisse"] if e["fotos"] > 0)
            assert all(e["fotos"] / e["seiten"] <= up["pro_seite_bis"] + 1e-9 for e in up["ereignisse"] if e["fotos"] > 0)
            assert sum(1 for e in up["ereignisse"] if e["fotos"] > 0 and e["links"]) >= len(up["ereignisse"]) - 2, "die meisten Ereignisse beginnen links"
            bild("umfang")
            vorher_buch = seite.evaluate("window.__fotobuch.S.a.zahlen.im_buch")
            seite.click(".ereigniskarte >> nth=0 >> .gfoto.drin >> nth=0")
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.im_buch === {vorher_buch - 1}")
            assert seite.locator(".ereigniskarte >> nth=0 >> .gfoto.eigen").count() == 1
            bild("umfang-auswahl", ziel=".ereigniskarte >> nth=0")
            seite.click(".ereigniskarte >> nth=0 >> .gfoto.eigen")
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.im_buch === {vorher_buch}")

            # Gestaltung: Themen ansehen, Leinen mit eigenem Titel wählen
            seite.evaluate("window.__fotobuch.geheZu(5)")
            seite.wait_for_selector("#gestaltvorschau .deckblatt")
            assert seite.locator(".themakarte").count() == 6
            assert seite.evaluate("window.__fotobuch.S.a.gestaltung.wahl.thema") == "klar"
            seite.wait_for_function("[...document.querySelectorAll('#themen img, #gestaltvorschau img')].every(i => i.complete && i.naturalWidth > 0)", timeout=30000)
            bild("gestaltung", voll=True)
            seite.click('.themakarte[data-thema="leinen"]')
            seite.wait_for_function("document.querySelector('.themakarte.aktiv')?.dataset.thema === 'leinen'")
            seite.fill("#g-titel", "New York 2026")
            seite.press("#g-titel", "Tab")
            seite.wait_for_function("window.__fotobuch.S.a.gestaltung.wahl.titel === 'New York 2026'")
            seite.fill("#g-name", "Familie Muster")
            seite.press("#g-name", "Tab")
            seite.wait_for_function("window.__fotobuch.S.a.gestaltung.wahl.name === 'Familie Muster'")
            seite.click("#titelfotos .titelfoto >> nth=2")
            seite.wait_for_function("window.__fotobuch.S.a.gestaltung.wahl.titelfoto !== ''")
            seite.wait_for_function("[...document.querySelectorAll('#themen img, #gestaltvorschau img')].every(i => i.complete && i.naturalWidth > 0)", timeout=30000)
            assert seite.inner_text("#gestaltvorschau .dk-titel") == "New York 2026"
            assert seite.locator("#gestaltvorschau .seite.th-leinen.rm-weiss").count() == 2
            assert seite.evaluate("window.__fotobuch.S.a.gestaltung.untertitel_auto").startswith("14.–17. Mai 2026"), seite.evaluate("window.__fotobuch.S.a.gestaltung.untertitel_auto")
            bild("gestaltung-leinen", ziel="#gestaltvorschau")

            # 3. Eine Entscheidung treffen: im Buch ein Foto gross ansehen und entfernen
            seite.evaluate("window.__fotobuch.geheZu(6)")
            seite.wait_for_selector("#doppelseite .sfoto")
            vor = seite.evaluate("window.__fotobuch.S.a.zahlen.im_buch")
            seite.click("#doppelseite .sfoto")
            seite.wait_for_selector("#lupe-grund")
            bild("foto-detail")
            seite.click("#lupe-umschalten")
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.im_buch === {vor - 1}")
            seite.keyboard.press("Escape")

            # Panoramen laufen über die Doppelseite, die Seitenzahl stimmt mit dem Zähler überein
            def buch_info():
                return seite.evaluate("({n: window.__fotobuch.S.a.buch.seiten.length, plan: window.__fotobuch.S.a.seitenplan.seiten_benoetigt, links: window.__fotobuch.S.a.buch.seiten.filter(s => s.art === 2).map(s => s.nr), ganz: window.__fotobuch.S.a.buch.seiten.filter(s => s.art === 1).length})")

            def zeige_seite(nr):
                seite.evaluate(f"window.__fotobuch.S.buchAnsicht = 'doppel'; window.__fotobuch.S.buchPos = Math.ceil(({nr} - 1) / 2); window.__fotobuch.zeige()")
                seite.wait_for_selector("#doppelseite .sfoto")
                seite.wait_for_function("[...document.querySelectorAll('#doppelseite img')].every(i => i.complete && i.naturalWidth > 0)")

            info = buch_info()
            assert info["n"] == info["plan"], info
            assert len(info["links"]) >= 2 and all(nr % 2 == 0 for nr in info["links"]), info
            pano_nr = info["links"][0]
            zeige_seite(pano_nr)
            assert seite.locator("#doppelseite .sfoto").count() == 2, "beide Hälften zeigen das Panorama"
            bild("panorama-ueber-doppelseite", ziel="#doppelseite")
            seite.click("#doppelseite .sfoto >> nth=0")
            seite.wait_for_selector("#lupe-art")
            assert seite.input_value("#lupe-art") == "doppel"
            seite.select_option("#lupe-art", "seite")
            seite.wait_for_function("window.__fotobuch.S.a.buch.seiten.filter(s => s.art === 1).length === " + str(info["ganz"] + 1))
            nach = buch_info()
            assert nach["n"] == nach["plan"], nach
            assert len(nach["links"]) == len(info["links"]) - 1
            zeige_seite(pano_nr)
            seite.click("#doppelseite .sfoto >> nth=0")
            seite.select_option("#lupe-art", "doppel")
            seite.wait_for_function(f"window.__fotobuch.S.a.buch.seiten.filter(s => s.art === 2).length === {len(info['links'])}")
            assert buch_info()["n"] == buch_info()["plan"]

            # Foto von Hand auf die nächste Seite legen, wieder zurück
            ziel_info = seite.evaluate("""(() => {
              const sp = window.__fotobuch.S.a.buch.seiten;
              for (let i = 3; i < sp.length - 1; i++) if (sp[i].art === 0 && sp[i + 1].art === 0 && sp[i].fotos.length >= 2) return {nr: sp[i].nr, id: sp[i].fotos[0].id, n: sp[i].fotos.length, n2: sp[i + 1].fotos.length};
              return null; })()""")
            assert ziel_info, "eine Seite zum Verschieben gefunden"
            zeige_seite(ziel_info["nr"])
            seite.click(f'#doppelseite .sfoto[data-id="{ziel_info["id"]}"]')
            seite.wait_for_selector("#lupe-nachher")
            bild("foto-verschieben")
            seite.click("#lupe-nachher")
            seite.wait_for_function(f"window.__fotobuch.S.a.buch.seiten.find(s => s.nr === {ziel_info['nr'] + 1}).fotos.some(f => f.id === '{ziel_info['id']}')")
            seite.wait_for_selector("#verschiebungen-zurueck", state="attached")
            assert seite.evaluate(f"window.__fotobuch.S.a.buch.seiten.find(s => s.nr === {ziel_info['nr']}).fotos.length") == ziel_info["n"] - 1
            zeige_seite(ziel_info["nr"])
            bild("foto-verschoben", ziel="#doppelseite")
            seite.click("#verschiebungen-zurueck")
            seite.wait_for_function(f"window.__fotobuch.S.a.buch.seiten.find(s => s.nr === {ziel_info['nr']}).fotos.length === {ziel_info['n']}")
            assert buch_info() == info or buch_info()["n"] == buch_info()["plan"]

            # Layout-Vorlagen und Übersicht für die Galerie
            seite.evaluate("window.__fotobuch.S.buchPos = 3; window.__fotobuch.zeige()")
            seite.click(".layout-knopf >> nth=0")
            seite.wait_for_selector(".layoutoption")
            bild("layout-vorlagen")
            # Aufklappfenster bleiben im Fenster und vergrössern nichts, auch bei kleinem Fenster und ganz oben oder unten
            def innerhalb():
                return seite.evaluate("""() => { const r = document.querySelector('.layoutwahl').getBoundingClientRect(); const d = document.documentElement;
                  return {ok: r.left >= 0 && r.top >= 0 && r.right <= d.clientWidth + 0.5 && r.bottom <= d.clientHeight + 0.5, r: [r.left, r.top, r.right, r.bottom], sb: d.scrollHeight, sw: d.scrollWidth, cw: d.clientWidth}; }""")
            for gross, (bw, bh) in {"normal": (1500, 950), "klein": (700, 480)}.items():
                seite.set_viewport_size({"width": bw, "height": bh})
                for rechts in (False, True):
                    seite.evaluate("window.scrollTo(0, 0)")
                    seite.wait_for_selector(".layoutwahl .layoutoption")
                    seite.evaluate("window.scrollTo(0, document.documentElement.scrollHeight)" if rechts else "window.scrollTo(0, 0)")
                    seite.wait_for_timeout(200)
                    m = innerhalb()
                    assert m["ok"], (gross, rechts, m)
                    assert m["sw"] <= m["cw"] + 1, ("kein seitliches Wachsen", m)
                    vorher_hoehe = seite.evaluate("document.documentElement.scrollHeight")
                    seite.wait_for_timeout(100)
                    assert seite.evaluate("document.documentElement.scrollHeight") == vorher_hoehe
            seite.set_viewport_size({"width": 1500, "height": 950})
            seite.evaluate("window.scrollTo(0, 0)")
            seite.keyboard.press("Escape")
            seite.wait_for_function("!document.querySelector('.layoutwahl')")
            seite.click(".layout-knopf >> nth=0")
            seite.wait_for_selector(".layoutwahl .layoutoption")
            seite.click("#layout-zu")
            seite.click("#btn-ansicht-uebersicht")
            seite.wait_for_selector("#uebersicht .seitenkarte")
            seite.wait_for_timeout(1500)
            bild("buch-uebersicht", voll=False)
            assert seite.locator("#band .bandseite").count() >= 3, "Seitenband zeigt die Seiten des Buchs"
            seite.evaluate("window.__fotobuch.S.buchAnsicht = 'doppel'; window.__fotobuch.zeige()")

            # 4. Buch durchblättern, jede Doppelseite prüfen und festhalten
            seite.evaluate("window.__fotobuch.geheZu(6)")
            seite.wait_for_selector("#doppelseite .sfoto")
            seite.evaluate("window.__fotobuch.S.buchPos = 0; window.__fotobuch.zeige()")
            bilder = []
            geprueft = 0
            while True:
                seite.wait_for_selector("#doppelseite .seite")
                seite.wait_for_function("[...document.querySelectorAll('#doppelseite img')].every(i => i.complete && i.naturalWidth > 0)", timeout=30000)
                n = seite.locator("#doppelseite .sfoto").count()
                assert n >= 1 or seite.locator("#doppelseite .seite.frei").count() >= 1, "Seite ohne Fotos nur als freie Seite"
                geprueft += n
                pfad = ausgabe / f"doppelseite-{len(bilder) + 1:02d}.png"
                seite.locator("#doppelseite").screenshot(path=str(pfad))
                bilder.append(pfad)
                if len(bilder) == 2:
                    bild("buch-doppelseite", ziel="#doppelseite")
                if seite.locator("#btn-buch-weiter").is_disabled():
                    break
                seite.click("#btn-buch-weiter")
            buch = seite.evaluate("window.__fotobuch.S.a.zahlen")
            assert geprueft == buch["im_buch"] + len(info["links"]), "jedes Buchfoto erscheint im Album, Doppelseiten-Fotos auf zwei Seiten"
            assert len(bilder) >= 10

            # 5. Export
            seite.evaluate("window.__fotobuch.geheZu(7)")
            ziel = basis / "Ausgabe"
            seite.fill("#ziel", str(ziel))
            seite.click("#btn-export")
            seite.wait_for_selector("#export-ergebnis", timeout=60000)
            im_buch_final = seite.evaluate("window.__fotobuch.S.a.zahlen.im_buch")
            assert f"{im_buch_final} Fotos kopiert" in seite.inner_text("#export-ergebnis")

            # Fertige Seiten (JPEG + PDF)
            plan_seiten = seite.evaluate("window.__fotobuch.S.a.seitenplan.buch_seiten")
            seite.click("#btn-seiten")
            seite.wait_for_selector("#seiten-ergebnis", timeout=300000)
            bild("export-fertige-seiten", voll=True)

            # Alle Themen exportieren und prüfen (Hintergrund, Deckblatt), je Thema ein Bild für die Galerie
            erwartet = {"klar": (255, 255, 255), "leinen": (251, 250, 246), "nacht": (19, 20, 23), "meer": (234, 243, 246), "galerie": (247, 247, 245)}
            themen_ordner = {"leinen": ziel / "seiten"}  # Leinen wurde oben mit eigenem Titel exportiert
            for t in [t for t in THEMEN if t != "leinen"]:
                seite.evaluate("window.__fotobuch.geheZu(5)")
                seite.click(f'.themakarte[data-thema="{t}"]')
                seite.wait_for_function(f"document.querySelector('.themakarte.aktiv')?.dataset.thema === '{t}'")
                seite.evaluate("window.__fotobuch.geheZu(7)")
                seite.fill("#ziel", str(basis / f"Thema-{t}"))
                seite.click("#btn-seiten")
                seite.wait_for_function(f"document.querySelector('#seiten-ergebnis')?.textContent.includes('Thema-{t}')", timeout=300000)
                themen_ordner[t] = basis / f"Thema-{t}" / "seiten"
            seite.click("#btn-beenden")
            browser.close()
    finally:
        proc.kill()

    assert not fehler, fehler
    # Export vollständig und Originale unverändert
    kopiert = [p for p in ziel.rglob("*") if p.is_file() and p.suffix.lower() in (".jpg", ".png") and "seiten" not in p.relative_to(ziel).parts[0]]
    assert len(kopiert) == im_buch_final
    assert (ziel / "seitenliste.csv").exists()
    assert pruefsummen(fotos) == vorher, "Originale unverändert"

    # Fertige Seiten: ein JPEG je Buchseite in voller Auflösung, PDF mit gleich vielen Seiten
    seiten_dir = ziel / "seiten"
    jpgs = sorted(seiten_dir.glob("seite-*.jpg"))
    assert len(jpgs) == plan_seiten, (len(jpgs), plan_seiten)
    with Image.open(jpgs[1]) as s2:
        assert s2.size == (3508, 2631)
        assert s2.convert("L").resize((64, 48)).getextrema()[0] < 200, "Seite enthält Fotos"
    pdf = (seiten_dir / "fotobuch.pdf").read_bytes()
    assert pdf.startswith(b"%PDF-1.4") and pdf.rstrip().endswith(b"%%EOF")
    assert pdf.count(b"/Type /Page ") == plan_seiten
    assert f"/Count {plan_seiten}".encode() in pdf

    # Deckblatt und Themen
    assert (seiten_dir / "deckblatt.jpg").exists() and (seiten_dir / "deckblatt.pdf").exists()
    with Image.open(seiten_dir / "deckblatt.jpg") as c:
        assert c.size == (3508, 2631)
        assert all(abs(a - b) < 25 for a, b in zip(c.convert("RGB").getpixel((5, 5)), (127, 148, 130))), "Deckblatt im gewählten Thema (Leinen)"
    galerie_dir = ausgabe / "galerie"
    nahe = lambda p, soll, tol=12: all(abs(a - b) <= tol for a, b in zip(p, soll))
    for t in THEMEN:
        d = themen_ordner[t]
        assert (d / "deckblatt.jpg").exists() and (d / "fotobuch.pdf").exists(), t
        seiten_t = sorted(d.glob("seite-*.jpg"))
        assert len(seiten_t) == plan_seiten, (t, len(seiten_t))
        with Image.open(d / "deckblatt.jpg") as c, Image.open(seiten_t[1]) as s2, Image.open(seiten_t[2]) as s3:
            c, s2, s3 = c.convert("RGB"), s2.convert("RGB"), s3.convert("RGB")
            assert c.size == s2.size == (3508, 2631)
            ecke = s2.getpixel((5, 5))
            if t in erwartet:
                assert nahe(ecke, erwartet[t]), (t, ecke)
            else:
                assert ecke[0] > ecke[2] + 30 and 150 < ecke[0] < 235, ("Kraftpapier", ecke)
            if t == "nacht":
                assert sum(c.getpixel((20, c.height - 20))) < 150, "Nacht: dunkler Fuss am Deckblatt"
            if t == "meer":
                assert sum(c.getpixel((5, 5))) > sum(c.getpixel((5, c.height - 5))) + 60, "Meer: Verlauf von hell nach dunkel"
            if t == "klar":
                assert nahe(c.getpixel((5, 5)), (255, 255, 255))
            drei = Image.new("RGB", (1800, 450), (90, 90, 90))
            for k, img in enumerate((c, s2, s3)):
                drei.paste(img.resize((600, 450)), (k * 600, 0))
            drei.save(galerie_dir / f"{20 + ALLE_THEMEN.index(t):02d}-thema-{t}.jpg", quality=88)

    # Neustart: Entscheidung und Ereignisname bleiben erhalten
    proc, url = starte(fotos)
    try:
        with sync_playwright() as pw:
            b = pw.chromium.launch()
            s = b.new_context(bypass_csp=True).new_page()
            s.goto(url)
            s.wait_for_function("window.__fotobuch && window.__fotobuch.S.a", timeout=60000)
            assert s.evaluate("window.__fotobuch.S.a.ereignisse[0].name") == "Ankunft"
            assert s.evaluate("window.__fotobuch.S.a.zahlen.im_buch") == im_buch_final
            b.close()
    finally:
        proc.kill()

    # Kontaktbogen: alle Doppelseiten als ein Album-Bild
    kacheln = [Image.open(p).convert("RGB") for p in bilder]
    breite = 480
    kacheln = [k.resize((breite, round(k.height * breite / k.width))) for k in kacheln]
    spalten = 4
    zeilen = -(-len(kacheln) // spalten)
    hoehe = max(k.height for k in kacheln)
    bogen = Image.new("RGB", (spalten * (breite + 12) + 12, zeilen * (hoehe + 12) + 12), (230, 232, 235))
    for i, k in enumerate(kacheln):
        bogen.paste(k, (12 + (i % spalten) * (breite + 12), 12 + (i // spalten) * (hoehe + 12)))
    bogen.save(ausgabe / "album-kontaktbogen.png")
    for k, j in enumerate(jpgs[:1] + jpgs[1:2]):
        Image.open(j).convert("RGB").resize((1200, 900)).save(ausgabe / "galerie" / f"99-fertige-seite-{k + 1}.jpg")
    print(f"Album: {ausgabe / 'album-kontaktbogen.png'}")
