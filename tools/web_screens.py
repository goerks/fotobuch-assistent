"""Erzeugt die Webseiten-Bilder: startet die echte App mit dem Demo-Set (tools/web_demo.py) und hält jeden Schritt fest.
Aufruf: python tools/web_screens.py <arbeitsordner> <zielordner-bilder>"""
import subprocess
import sys
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

W = Path(__file__).resolve().parent.parent
BIN = W / "target" / "release" / "fotobuch-dev"
THEMEN = ["klar", "leinen", "nacht", "kraft", "meer", "galerie"]


def main(arbeit: Path, ziel: Path):
    arbeit.mkdir(parents=True, exist_ok=True); ziel.mkdir(parents=True, exist_ok=True)
    fotos = arbeit / "Lago-Maggiore-2026"
    if not fotos.exists():
        subprocess.run([sys.executable, str(W / "tools" / "web_demo.py"), str(fotos)], check=True)
    import shutil
    shutil.rmtree(fotos / ".fotobuch", ignore_errors=True)  # frischer Start
    p = subprocess.Popen([str(BIN), "serve", str(fotos)], stdout=subprocess.PIPE, text=True)
    url = p.stdout.readline().strip()
    try:
        with sync_playwright() as pw:
            b = pw.chromium.launch()
            s = b.new_context(viewport={"width": 1360, "height": 860}, bypass_csp=True, device_scale_factor=1).new_page()
            s.goto(url); s.wait_for_function("window.__fotobuch && window.__fotobuch.S.a")

            def warte_bilder():
                try:
                    s.wait_for_function("[...document.querySelectorAll('img')].filter(i => { const r = i.getBoundingClientRect(); return r.bottom > 0 && r.top < innerHeight; }).every(i => i.complete && i.naturalWidth > 0)", timeout=20000)
                except Exception:
                    pass
                s.wait_for_timeout(300)

            def shot(name, ziel_sel=None, voll=False):
                warte_bilder()
                t = s.locator(ziel_sel) if ziel_sel else s
                t.screenshot(path=str(ziel / f"{name}.png")) if ziel_sel else s.screenshot(path=str(ziel / f"{name}.png"), full_page=voll)

            def gehe(n):
                s.evaluate(f"window.__fotobuch.geheZu({n})"); s.wait_for_selector("h1"); s.evaluate("window.scrollTo(0,0)")

            shot("1-start")
            s.click("#btn-analyse")
            s.wait_for_function("/Ereignisse/.test(document.querySelector('h1')?.textContent)", timeout=120000)
            shot("2-ereignisse")
            gehe(3); shot("3-aussortieren")
            s.evaluate("document.querySelector('#orte, .ort')?.scrollIntoView()"); shot("3b-orte")
            gehe(4); shot("4-seiten")
            gehe(5); s.wait_for_selector("#gestaltvorschau .deckblatt"); shot("5-gestaltung", voll=True)
            s.fill("#g-titel", "Sommer am Lago Maggiore"); s.press("#g-titel", "Tab")
            s.fill("#g-name", "Familie Muster"); s.press("#g-name", "Tab")
            s.click("#titelfotos .titelfoto >> nth=3"); s.wait_for_timeout(600)
            for t in THEMEN:
                s.click(f'.themakarte[data-thema="{t}"]')
                s.wait_for_function(f"document.querySelector('.themakarte.aktiv')?.dataset.thema === '{t}'")
                warte_bilder(); shot(f"thema-vorschau-{t}", "#gestaltvorschau")
            s.click('.themakarte[data-thema="leinen"]'); s.wait_for_timeout(500)
            gehe(6); s.wait_for_selector("#doppelseite .sfoto")
            s.evaluate("window.__fotobuch.S.buchAnsicht='doppel'; window.__fotobuch.S.buchPos=2; window.__fotobuch.zeige()")
            shot("6-buch")
            s.click(".layout-knopf >> nth=0"); s.wait_for_selector(".layoutoption"); shot("6b-layouts"); s.keyboard.press("Escape")
            pano = s.evaluate("window.__fotobuch.S.a.buch.seiten.filter(x => x.art === 2).map(x => x.nr)[0]")
            if pano:
                s.evaluate(f"window.__fotobuch.S.buchPos=Math.ceil(({pano}-1)/2); window.__fotobuch.zeige()")
                s.wait_for_selector("#doppelseite .sfoto"); shot("6c-panorama")
            s.click("#doppelseite .sfoto >> nth=0"); s.wait_for_selector("#lupe-grund"); shot("6d-foto-detail"); s.keyboard.press("Escape")
            s.click("#btn-ansicht-uebersicht"); s.wait_for_selector("#uebersicht .seitenkarte"); s.wait_for_timeout(1500); shot("6e-uebersicht")
            s.evaluate("window.__fotobuch.S.buchAnsicht='doppel'; window.__fotobuch.S.buchPos=3; window.__fotobuch.zeige()")
            s.wait_for_selector("#band .bandseite"); s.evaluate("document.querySelector('#seitenband').scrollIntoView()"); shot("6f-seitenband")
            s.evaluate("window.scrollTo(0,0)")
            gehe(7)
            for t in THEMEN:
                gehe(5); s.click(f'.themakarte[data-thema="{t}"]')
                s.wait_for_function(f"document.querySelector('.themakarte.aktiv')?.dataset.thema === '{t}'")
                gehe(7); out = arbeit / f"ausgabe-{t}"
                s.fill("#ziel", str(out)); s.click("#btn-seiten")
                s.wait_for_function(f"document.querySelector('#seiten-ergebnis')?.textContent.includes('ausgabe-{t}')", timeout=300000)
                if t == "leinen":
                    shot("7-export", voll=True)
            s.click("#btn-beenden"); b.close()
    finally:
        p.kill()
    # Echte Buchseiten je Thema (Deckblatt + zwei Seiten) in Webgrösse
    for t in THEMEN:
        d = arbeit / f"ausgabe-{t}" / "seiten"
        seiten = sorted(d.glob("seite-*.jpg"))
        for name, quelle in (("deckblatt", d / "deckblatt.jpg"), ("seite-a", seiten[1]), ("seite-b", seiten[4])):
            Image.open(quelle).convert("RGB").resize((1000, 750), Image.Resampling.LANCZOS).save(ziel / f"buch-{t}-{name}.jpg", quality=82, optimize=True)
    print("fertig", len(list(ziel.iterdir())), "Bilder")


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]))
