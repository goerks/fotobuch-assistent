"""Erzeugt das Demo-Fotoset für die Webseite: genau 100 einfache, gezeichnete «Fotos» einer Ferienreise
(Sommerferien am Lago Maggiore) mit EXIF-Zeit, GPS, Serien, Duplikaten, Screenshots, Panoramen und Fotos ohne Zeit.
Es sind synthetische Bilder, keine echten Fotos.
Aufruf: python tools/web_demo.py <zielordner>"""
import math
import random
import sys
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tests"))
from conftest import speichere  # noqa: E402


def verlauf(w, h, oben, unten, bis=1.0):
    t = np.clip(np.linspace(0, 1, h)[:, None] / bis, 0, 1)
    a = np.array(oben)[None, None, :] * (1 - t[..., None]) + np.array(unten)[None, None, :] * t[..., None]
    return Image.fromarray(np.broadcast_to(a, (h, w, 3)).astype(np.uint8))


def himmel(w, h, stunde, bis=0.62):
    if stunde < 10:
        o, u = (150, 195, 235), (255, 235, 200)
    elif stunde < 17:
        o, u = (70, 140, 215), (200, 228, 245)
    else:
        o, u = (60, 70, 150), (255, 170, 110)
    return verlauf(w, h, o, u, bis)


def sonne(d, w, h, r, stunde, y=0.25):
    x = int(w * (0.15 + 0.7 * r.random()))
    farbe = (255, 244, 205) if stunde < 17 else (255, 205, 140)
    for k in range(5, 0, -1):
        rad = int(h * 0.04 * k)
        d.ellipse([x - rad, int(h * y) - rad, x + rad, int(h * y) + rad], fill=farbe + (0,) if False else farbe)
    return x


def berge(d, w, h, r, farben, basis, hoehe, schnee=False):
    for lage, farbe in enumerate(farben):
        b = int(h * (basis + 0.07 * lage))
        pts, x = [(0, h)], -40
        while x <= w + 40:
            gipfel = b - r.randint(int(hoehe * 0.3 * h), int(hoehe * h)) * (1 - 0.18 * lage)
            pts += [(x, b), (x + r.randint(60, 160), gipfel)]
            x += r.randint(140, 300)
        pts += [(w, h)]
        d.polygon(pts, fill=farbe)
        if schnee and lage == 0:
            for (px, py) in pts[2::2]:
                d.polygon([(px - 28, py + 34), (px, py), (px + 28, py + 34), (px + 8, py + 26), (px - 6, py + 38)], fill=(246, 248, 252))


def tanne(d, x, y, s, farbe=(34, 78, 56)):
    d.rectangle([x - s * 0.06, y, x + s * 0.06, y + s * 0.22], fill=(90, 62, 40))
    for k in range(3):
        b = s * (0.5 - 0.1 * k)
        yy = y - s * 0.3 * k
        d.polygon([(x - b / 2, yy), (x + b / 2, yy), (x, yy - s * 0.5)], fill=farbe)


def haus(d, x, y, bw, bh, farbe, dach, r):
    d.rectangle([x, y - bh, x + bw, y], fill=farbe)
    d.polygon([(x - 6, y - bh), (x + bw + 6, y - bh), (x + bw / 2, y - bh - bw * 0.35)], fill=dach)
    for fx in range(int(x + 10), int(x + bw - 18), 34):
        for fy in range(int(y - bh + 14), int(y - 30), 40):
            d.rectangle([fx, fy, fx + 18, fy + 24], fill=(250, 240, 190) if r.random() < 0.7 else (70, 90, 120))
    d.rectangle([x + bw / 2 - 10, y - 34, x + bw / 2 + 10, y], fill=(110, 70, 45))


def szene_strand(w, h, r, st):
    img = himmel(w, h, st); d = ImageDraw.Draw(img)
    sonne(d, w, h, r, st)
    see = verlauf(w, int(h * 0.2), (40, 150, 175), (90, 200, 205))
    img.paste(see, (0, int(h * 0.55)))
    for _ in range(26):
        x, y = r.randint(0, w), r.randint(int(h * 0.57), int(h * 0.73))
        d.arc([x, y, x + r.randint(40, 90), y + 14], 200, 340, fill=(235, 250, 250), width=3)
    sand = verlauf(w, h - int(h * 0.75), (240, 220, 175), (222, 190, 140))
    img.paste(sand, (0, int(h * 0.75)))
    d.polygon([(0, int(h * 0.75)), (w, int(h * 0.73)), (w, int(h * 0.76)), (0, int(h * 0.78))], fill=(250, 240, 215))
    for _ in range(r.randint(1, 3)):
        x, y = r.randint(int(w * 0.1), int(w * 0.85)), r.randint(int(h * 0.78), int(h * 0.92))
        f = [(230, 70, 70), (250, 190, 60), (60, 130, 210)][r.randint(0, 2)]
        d.rectangle([x - 2, y - 120, x + 2, y + 6], fill=(80, 60, 50))
        d.pieslice([x - 85, y - 160, x + 85, y - 40], 180, 360, fill=f)
        for k in range(-2, 3):
            d.line([(x, y - 160), (x + k * 40, y - 100)], fill=(255, 255, 255), width=2)
    return img


def szene_berge(w, h, r, st):
    img = himmel(w, h, st, 0.7); d = ImageDraw.Draw(img)
    sonne(d, w, h, r, st, 0.2)
    berge(d, w, h, r, [(104, 124, 158), (74, 104, 120), (52, 86, 82)], 0.55, 0.26, schnee=True)
    see = verlauf(w, int(h * 0.3), (70, 130, 150), (40, 90, 110))
    img.paste(see, (0, int(h * 0.72)))
    for x in range(0, w, r.randint(46, 70)):
        tanne(d, x + r.randint(-12, 12), int(h * 0.72) + r.randint(-10, 6), r.randint(110, 190))
    return img


def szene_stadt(w, h, r, st):
    img = himmel(w, h, st, 0.55); d = ImageDraw.Draw(img)
    sonne(d, w, h, r, st, 0.18)
    d.rectangle([0, int(h * 0.62), w, h], fill=(86, 150, 175))
    x = -20
    farben = [(240, 190, 120), (230, 120, 110), (250, 220, 150), (150, 190, 200), (235, 160, 140), (200, 210, 160)]
    while x < w:
        bw = r.randint(150, 240)
        haus(d, x, int(h * 0.64), bw, r.randint(150, 270), farben[r.randint(0, 5)], (170, 80, 60), r)
        x += bw + r.randint(4, 14)
    d.rectangle([0, int(h * 0.64), w, int(h * 0.7)], fill=(205, 198, 188))
    for k in range(0, w, 120):
        d.ellipse([k + 20, int(h * 0.64) - 6, k + 60, int(h * 0.64) + 18], fill=(60, 110, 70))
    return img


def szene_see(w, h, r, st):
    img = himmel(w, h, st, 0.5); d = ImageDraw.Draw(img)
    sx = sonne(d, w, h, r, st, 0.2)
    berge(d, w, h, r, [(98, 128, 150), (70, 105, 112)], 0.5, 0.2)
    see = verlauf(w, h - int(h * 0.5), (60, 140, 175), (30, 90, 135))
    img.paste(see, (0, int(h * 0.5)))
    for _ in range(18):
        y = r.randint(int(h * 0.55), h - 10)
        d.line([(sx - r.randint(20, 90), y), (sx + r.randint(20, 90), y)], fill=(255, 240, 200), width=3)
    bx, by = r.randint(int(w * 0.25), int(w * 0.65)), int(h * 0.72)
    d.polygon([(bx - 110, by), (bx + 120, by), (bx + 80, by + 40), (bx - 70, by + 40)], fill=(250, 250, 248))
    d.rectangle([bx - 3, by - 190, bx + 3, by], fill=(90, 70, 60))
    d.polygon([(bx + 8, by - 185), (bx + 8, by - 8), (bx + 110, by - 8)], fill=(255, 245, 235))
    d.polygon([(bx - 8, by - 150), (bx - 8, by - 8), (bx - 85, by - 8)], fill=(230, 90, 80))
    return img


def szene_wald(w, h, r, st):
    img = verlauf(w, h, (170, 205, 170), (60, 100, 70)); d = ImageDraw.Draw(img)
    d.polygon([(w * 0.42, h * 0.55), (w * 0.58, h * 0.55), (w * 0.9, h), (w * 0.1, h)], fill=(190, 165, 120))
    for _ in range(14):
        x = r.randint(-20, w)
        if w * 0.35 < x < w * 0.65:
            continue
        bw = r.randint(26, 60)
        d.rectangle([x, 0, x + bw, h], fill=(78 + r.randint(-10, 10), 56, 38))
        for _ in range(4):
            cx, cy = x + bw / 2 + r.randint(-70, 70), r.randint(0, int(h * 0.5))
            rad = r.randint(90, 160)
            d.ellipse([cx - rad, cy - rad, cx + rad, cy + rad], fill=(52 + r.randint(0, 40), 120 + r.randint(0, 50), 64))
    for k in range(7):
        d.polygon([(w * (0.2 + 0.1 * k), 0), (w * (0.22 + 0.1 * k), 0), (w * (0.1 + 0.1 * k), h * 0.9), (w * (0.04 + 0.1 * k), h * 0.9)], fill=(255, 255, 220))
    return Image.blend(img, verlauf(w, h, (170, 205, 170), (60, 100, 70)), 0.0)


def szene_cafe(w, h, r, st):
    img = Image.new("RGB", (w, h), (246, 240, 228)); d = ImageDraw.Draw(img)
    for x in range(0, w, 80):
        for y in range(0, h, 80):
            if (x // 80 + y // 80) % 2 == 0:
                d.rectangle([x, y, x + 80, y + 80], fill=(222, 70, 66))
    cx, cy = int(w * 0.5), int(h * 0.5)
    d.ellipse([cx - 230, cy - 200, cx + 230, cy + 220], fill=(0, 0, 0, 0) if False else (200, 180, 165))
    d.ellipse([cx - 220, cy - 215, cx + 220, cy + 205], fill=(252, 252, 250))
    d.ellipse([cx - 150, cy - 140, cx + 150, cy + 130], fill=(238, 238, 232))
    for _ in range(9):
        a = r.random() * 6.28; rr = r.randint(10, 100)
        d.ellipse([cx + math.cos(a) * rr - 28, cy + math.sin(a) * rr - 28, cx + math.cos(a) * rr + 28, cy + math.sin(a) * rr + 28],
                  fill=[(232, 90, 60), (110, 170, 70), (250, 200, 70), (245, 230, 200)][r.randint(0, 3)])
    d.ellipse([w - 330, 60, w - 130, 260], fill=(250, 250, 250)); d.ellipse([w - 300, 90, w - 160, 230], fill=(92, 58, 38))
    d.ellipse([60, h - 220, 220, h - 60], fill=(255, 215, 80)); d.line([(80, h - 140), (200, h - 140)], fill=(255, 245, 190), width=5)
    return img


def szene_garten(w, h, r, st):
    img = verlauf(w, h, (170, 215, 235), (210, 235, 200), 0.4); d = ImageDraw.Draw(img)
    d.rectangle([0, int(h * 0.38), w, h], fill=(96, 170, 98))
    d.polygon([(w * 0.4, h * 0.4), (w * 0.6, h * 0.4), (w * 0.95, h), (w * 0.05, h)], fill=(228, 212, 170))
    for _ in range(90):
        x, y = r.randint(0, w), r.randint(int(h * 0.42), h)
        if abs(x - w / 2) < (y - h * 0.4) * 0.8:
            continue
        s = 6 + (y - h * 0.4) / h * 26
        f = [(250, 120, 160), (255, 200, 70), (180, 120, 230), (255, 255, 255)][r.randint(0, 3)]
        for a in range(5):
            d.ellipse([x + math.cos(a * 1.26) * s - s * 0.6, y + math.sin(a * 1.26) * s - s * 0.6, x + math.cos(a * 1.26) * s + s * 0.6, y + math.sin(a * 1.26) * s + s * 0.6], fill=f)
        d.ellipse([x - s * 0.4, y - s * 0.4, x + s * 0.4, y + s * 0.4], fill=(250, 220, 90))
    for _ in range(5):
        tanne(d, r.randint(0, w), int(h * 0.42), r.randint(180, 260), (44, 108, 66))
    return img


def person(d, x, y, s, farbe, haar):
    d.rounded_rectangle([x - s * 0.5, y, x + s * 0.5, y + s * 1.5], int(s * 0.3), fill=farbe)
    d.ellipse([x - s * 0.38, y - s * 0.85, x + s * 0.38, y - s * 0.05], fill=(240, 200, 170))
    d.pieslice([x - s * 0.42, y - s * 0.95, x + s * 0.42, y - s * 0.2], 180, 360, fill=haar)
    d.ellipse([x - s * 0.17, y - s * 0.52, x - s * 0.08, y - s * 0.42], fill=(40, 40, 50))
    d.ellipse([x + s * 0.08, y - s * 0.52, x + s * 0.17, y - s * 0.42], fill=(40, 40, 50))
    d.arc([x - s * 0.2, y - s * 0.5, x + s * 0.2, y - s * 0.15], 20, 160, fill=(150, 60, 60), width=3)


def szene_gruppe(w, h, r, st):
    img = szene_berge(w, h, r, st); d = ImageDraw.Draw(img)
    zs = [(0.35, (220, 90, 80), (60, 40, 30)), (0.5, (70, 120, 190), (200, 150, 70)), (0.65, (250, 200, 80), (30, 30, 35))]
    for fx, fa, ha in zs[: r.randint(2, 3)]:
        person(d, int(w * fx + r.randint(-20, 20)), int(h * 0.62), int(h * 0.22), fa, ha)
    return img


SZENEN = {"strand": szene_strand, "berge": szene_berge, "stadt": szene_stadt, "see": szene_see, "wald": szene_wald,
          "cafe": szene_cafe, "garten": szene_garten, "gruppe": szene_gruppe}


def foto(art, seed, stunde, hoch=False, breit=False):
    r = random.Random(seed)
    w, h = (1600, 1200) if not hoch else (1200, 1600)
    if breit:
        w, h = 3000, 1000
    img = SZENEN[art](w, h, r, stunde)
    a = np.asarray(img, dtype=np.float32) + np.random.default_rng(seed).normal(0, 3.5, (h, w, 3))
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.7))


def variante(img, nr, unscharf=False):
    v = img.transform(img.size, Image.Transform.AFFINE, (1, 0, nr * 9, 0, 1, nr * 5), resample=Image.Resampling.BICUBIC)
    a = np.asarray(v, dtype=np.float32) * (1 + 0.025 * nr)
    v = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))
    return v.filter(ImageFilter.GaussianBlur(5)) if unscharf else v


def screenshot(nr):
    img = Image.new("RGB", (1170, 2532), (245, 245, 247)); d = ImageDraw.Draw(img); r = random.Random(nr)
    for i in range(12):
        y = 160 + i * 175
        d.rounded_rectangle([60, y, 1110, y + 130], 26, fill=(255, 255, 255))
        d.ellipse([84, y + 24, 164, y + 104], fill=(190, 205, 225))
        d.rectangle([190, y + 36, 190 + r.randint(300, 760), y + 62], fill=(200, 200, 205))
        d.rectangle([190, y + 76, 190 + r.randint(200, 500), y + 94], fill=(225, 225, 228))
    return img


# Tag, Start, [(Name, Szene, Breite, Länge, Anzahl Dateien, Dauer Minuten)]
TAGE = [
    (datetime(2026, 7, 18, 14, 10), [("Promenade Ascona", "stadt", 46.1553, 8.7702, 12, 50), ("Café an der Piazza", "cafe", 46.1558, 8.7710, 7, 30), ("Strand Lido", "strand", 46.1590, 8.7740, 12, 75)]),
    (datetime(2026, 7, 19, 9, 20), [("Bergbahn Cardada", "berge", 46.2003, 8.8004, 14, 45), ("Waldweg", "wald", 46.1980, 8.7990, 8, 40), ("Gipfelrast", "gruppe", 46.2010, 8.8010, 9, 25)]),
    (datetime(2026, 7, 20, 10, 0), [("Schiff zu den Inseln", "see", 46.1480, 8.7600, 9, 40), ("Isole di Brissago", "garten", 46.1264, 8.7197, 13, 70), ("Rückfahrt", "see", 46.1500, 8.7650, 5, 25)]),
]
ZIEL_ANZAHL = 100


def main(ziel: Path) -> None:
    ziel.mkdir(parents=True, exist_ok=True)
    zufall = random.Random(5)
    nr = 0
    extra = 2 + 2 + 3 + 4  # Panoramen, Screenshots, Chat-Kopien, Fotos ohne Zeit
    for ti, (start, orte) in enumerate(TAGE):
        zeit = start
        for ort, art, lat, lon, n, dauer in orte:
            i, schritt = 0, max(1.0, dauer / n)
            while i < n:
                nr += 1
                zeit += timedelta(minutes=zufall.uniform(0.6 * schritt, 1.4 * schritt))
                hoch = art in ("stadt", "garten", "wald") and zufall.random() < 0.25
                handy = zufall.random() < 0.75
                basis = foto(art, 100 + nr, zeit.hour + zeit.minute / 60, hoch)
                marke, modell = ("Apple", "iPhone 15") if handy else ("Canon", "EOS R50")
                name = f"IMG_{4000 + nr}.jpg" if handy else f"_MG_{6000 + nr}.jpg"
                pos = (lat + zufall.gauss(0, 0.0003), lon + zufall.gauss(0, 0.0004)) if handy else None
                if zufall.random() < 0.2 and n - i >= 2:
                    k = min(zufall.randint(2, 3), n - i)
                    weg = zufall.randrange(1, k)
                    for j in range(k):
                        speichere(variante(basis, j, unscharf=(j == weg and k > 2)), ziel / f"{name[:-4]}_{j}.jpg", zeit + timedelta(seconds=j * 3),
                                  marke=marke, modell=modell, gps=pos, quality=90)
                    i += k
                else:
                    speichere(basis, ziel / name, zeit, marke=marke, modell=modell, gps=pos, quality=90)
                    i += 1
            zeit += timedelta(minutes=zufall.randint(15, 35))
    dateien = sorted(p.name for p in ziel.glob("IMG_*.jpg"))
    for nm in dateien[10:13]:  # Chat-Kopien
        (ziel / f"WhatsApp {nm}").write_bytes((ziel / nm).read_bytes())
    for k in range(2):
        screenshot(k).save(ziel / f"Screenshot_2026-07-{18 + k}.png")
    for k, (art, lat, lon) in enumerate([("see", 46.16, 8.78), ("berge", 46.2, 8.8)]):
        speichere(foto(art, 900 + k, 16 + k, breit=True), ziel / f"PANO_{7000 + k}.jpg", TAGE[1 + k][0] + timedelta(hours=4, minutes=11 + k),
                  marke="Apple", modell="iPhone 15", gps=(lat, lon), quality=90)
    for j in range(4):
        foto(["strand", "stadt", "cafe", "garten"][j], 950 + j, 12).save(ziel / f"gespeichert_{j}.jpg", quality=85)
    n = len(list(ziel.iterdir()))
    print(f"{n} Dateien in {ziel}")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
