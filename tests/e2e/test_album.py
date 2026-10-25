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
            ereignisse_auto = z["ereignisse"]

            # 2. Ereignisse benennen, zusammenlegen, teilen, zurücksetzen
            seite.fill('input[data-key] >> nth=0', "Ankunft")
            seite.press('input[data-key] >> nth=0', "Tab")
            seite.wait_for_function("window.__fotobuch.S.a.ereignisse[0].name === 'Ankunft'")
            seite.click('button[data-aktion="zusammenlegen"] >> nth=0')
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.ereignisse === {ereignisse_auto - 1}")
            seite.click('button[data-aktion="teilen"] >> nth=0')
            seite.click(".teilen-foto >> nth=10")
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.ereignisse === {ereignisse_auto}")
            seite.click("#ereignisse-zurueck")
            seite.wait_for_function("!window.__fotobuch.S.a.zahlen.ereignis_aenderungen")
            assert seite.evaluate("window.__fotobuch.S.a.zahlen.ereignisse") == ereignisse_auto
            assert seite.evaluate("window.__fotobuch.S.a.ereignisse[0].name") == "Ankunft", "Name bleibt bestehen"

            # 3. Eine Entscheidung treffen: im Buch ein Foto gross ansehen und entfernen
            seite.evaluate("window.__fotobuch.geheZu(5)")
            seite.wait_for_selector("#doppelseite .sfoto")
            vor = seite.evaluate("window.__fotobuch.S.a.zahlen.im_buch")
            seite.click("#doppelseite .sfoto")
            seite.click("#lupe-umschalten")
            seite.wait_for_function(f"window.__fotobuch.S.a.zahlen.im_buch === {vor - 1}")
            seite.keyboard.press("Escape")

            # 4. Buch durchblättern, jede Doppelseite prüfen und festhalten
            seite.evaluate("window.__fotobuch.geheZu(5)")
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
                if seite.locator("#btn-buch-weiter").is_disabled():
                    break
                seite.click("#btn-buch-weiter")
            buch = seite.evaluate("window.__fotobuch.S.a.zahlen")
            assert geprueft == buch["im_buch"], "jedes Buchfoto erscheint genau einmal im Album"
            assert len(bilder) >= 10

            # 5. Export
            seite.evaluate("window.__fotobuch.geheZu(6)")
            ziel = basis / "Ausgabe"
            seite.fill("#ziel", str(ziel))
            seite.click("#btn-export")
            seite.wait_for_selector("#export-ergebnis", timeout=60000)
            im_buch_final = seite.evaluate("window.__fotobuch.S.a.zahlen.im_buch")
            assert f"{im_buch_final} Fotos kopiert" in seite.inner_text("#export-ergebnis")
            seite.click("#btn-beenden")
            browser.close()
    finally:
        proc.kill()

    assert not fehler, fehler
    # Export vollständig und Originale unverändert
    kopiert = [p for p in ziel.rglob("*") if p.is_file() and p.suffix.lower() in (".jpg", ".png")]
    assert len(kopiert) == im_buch_final
    assert (ziel / "seitenliste.csv").exists()
    assert pruefsummen(fotos) == vorher, "Originale unverändert"

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
    print(f"Album: {ausgabe / 'album-kontaktbogen.png'}")
