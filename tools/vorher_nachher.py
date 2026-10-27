"""Erzeugt ein Vergleichsbild: links vorher, in der Mitte die Erklärung, rechts nachher.
Aufruf: python tools/vorher_nachher.py <vorher.png> <nachher.png> <ausgabe.png> <titel> <text1> [text2 ...]"""
import base64
import sys
from html import escape
from pathlib import Path

from playwright.sync_api import sync_playwright


def datei(p):
    return "data:image/png;base64," + base64.b64encode(Path(p).read_bytes()).decode()


def main(vorher, nachher, aus, titel, texte):
    punkte = "".join(f"<li>{escape(t)}</li>" for t in texte)
    html = f"""<!doctype html><meta charset=utf-8><style>
body{{margin:0;background:#e9ebf0;font-family:system-ui,'Segoe UI',sans-serif;color:#1c2330;padding:28px 32px;width:2400px;box-sizing:border-box}}
h1{{margin:0 0 18px;font-size:34px}} .zeile{{display:flex;gap:26px;align-items:flex-start}}
.bild{{flex:0 0 900px}} .bild img{{width:100%;display:block;border:1px solid #c3c9d4;border-radius:10px;box-shadow:0 8px 24px rgba(20,30,50,.15)}}
.etikett{{display:inline-block;color:#fff;font-weight:700;padding:3px 14px;border-radius:999px;margin-bottom:10px;font-size:18px}}
.mitte{{flex:1;background:#1f4e8c;color:#fff;border-radius:14px;padding:22px 26px;font-size:21px;line-height:1.45;margin-top:44px}}
.mitte b{{display:block;font-size:14px;letter-spacing:.07em;text-transform:uppercase;opacity:.8;margin-bottom:10px}}
.mitte ul{{margin:0;padding-left:22px;display:flex;flex-direction:column;gap:12px}}</style>
<h1>{escape(titel)}</h1><div class=zeile>
<div class=bild><span class=etikett style="background:#4b5565">VORHER</span><img src="{datei(vorher)}"></div>
<div class=mitte><b>Was ist jetzt besser?</b><ul>{punkte}</ul></div>
<div class=bild><span class=etikett style="background:#1f7a4d">NACHHER</span><img src="{datei(nachher)}"></div></div>"""
    with sync_playwright() as pw:
        b = pw.chromium.launch(); s = b.new_page(viewport={"width": 2400, "height": 800}); s.set_content(html); s.wait_for_timeout(500)
        s.screenshot(path=aus, full_page=True); b.close()


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:])
