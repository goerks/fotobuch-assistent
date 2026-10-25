"""Erzeugt einen realistischen Demo-Fotoordner (synthetische Bilder, keine echten Fotos).

Nachgebildet ist eine Städtereise nach New York über vier Tage mit GPS-Orten (Handy mit GPS, Spiegelreflex ohne): Serienbilder, Duplikate aus Messenger-Kopien,
Screenshots, Hoch- und Querformat, zwei Kameras, zwei Panoramen, einige Fotos ohne Aufnahmezeit.
Aufruf: python tools/demo_fotos.py <zielordner>
"""
import random
import sys
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tests"))
from conftest import speichere  # noqa: E402


def szene(seed: int, stunde: float, hoch: bool = False) -> Image.Image:
    r = random.Random(seed)
    w, h = (1200, 1600) if hoch else (1600, 1200)
    t = np.clip((stunde - 6) / 14, 0, 1)
    himmel_oben = np.array([40 + 90 * np.sin(np.pi * t), 90 + 110 * np.sin(np.pi * t), 150 + 90 * np.sin(np.pi * t)])
    himmel_unten = np.array([230, 180 + 40 * (1 - abs(2 * t - 1)), 140 + 90 * np.sin(np.pi * t)])
    y = np.linspace(0, 1, h)[:, None, None]
    bild = himmel_oben * (1 - y) + himmel_unten * y
    bild = np.broadcast_to(bild, (h, w, 3)).copy()
    img = Image.fromarray(np.clip(bild, 0, 255).astype(np.uint8))
    d = ImageDraw.Draw(img)
    sx, sy = r.randint(w // 6, 5 * w // 6), int(h * (0.15 + 0.5 * abs(2 * t - 1)))
    d.ellipse([sx - 50, sy - 50, sx + 50, sy + 50], fill=(255, 240, 200))
    for lage, farbe in enumerate([(70, 90, 110), (45, 70, 60), (30, 45, 35)]):
        basis = int(h * (0.55 + 0.12 * lage))
        pts = [(0, h)]
        x = 0
        while x <= w:
            pts.append((x, basis - r.randint(0, int(h * 0.12))))
            x += r.randint(40, 140)
        pts += [(w, h)]
        d.polygon(pts, fill=farbe)
    for _ in range(r.randint(2, 7)):  # Häuser / Personen
        bx, bw, bh = r.randint(0, w - 120), r.randint(40, 120), r.randint(80, 320)
        farbe = tuple(r.randint(60, 220) for _ in range(3))
        d.rectangle([bx, h - bh - 40, bx + bw, h - 40], fill=farbe)
        for fy in range(h - bh - 20, h - 60, 40):
            d.rectangle([bx + 10, fy, bx + bw - 10, fy + 20], fill=(250, 235, 160))
    a = np.asarray(img, dtype=np.float32)
    a += np.random.default_rng(seed).normal(0, 6, a.shape)
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.6))


def variante(img: Image.Image, nr: int, unscharf: bool = False) -> Image.Image:
    """Serienbild: leicht verschoben, andere Helligkeit, evtl. verwackelt."""
    dx, dy = nr * 7, nr * 4
    v = img.transform(img.size, Image.Transform.AFFINE, (1, 0, dx, 0, 1, dy), resample=Image.Resampling.BICUBIC)
    a = np.asarray(v, dtype=np.float32) * (1 + 0.03 * nr)
    v = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))
    return v.filter(ImageFilter.GaussianBlur(4)) if unscharf else v


def screenshot(seed: int) -> Image.Image:
    img = Image.new("RGB", (1170, 2532), (245, 245, 247))
    d = ImageDraw.Draw(img)
    r = random.Random(seed)
    for i in range(14):
        y = 150 + i * 160
        d.rounded_rectangle([60, y, 1110, y + 120], 24, fill=(255, 255, 255))
        d.rectangle([100, y + 40, 100 + r.randint(300, 800), y + 70], fill=(200, 200, 205))
    return img


# Städtereise New York: Tag, Startzeit, Orte mit (Name, Breite, Länge, Anzahl Fotos, Dauer in Minuten)
TAGE = [
    ("Ankunft", datetime(2026, 5, 14, 14, 20), [
        ("Times Square", 40.7580, -73.9855, 52, 70),
        ("Bryant Park", 40.7536, -73.9832, 12, 25),
    ]),
    ("Downtown", datetime(2026, 5, 15, 9, 30), [
        ("Battery Park Fähre", 40.7033, -74.0170, 24, 40),
        ("Brooklyn Bridge", 40.7061, -73.9969, 46, 75),
        ("DUMBO", 40.7033, -73.9890, 18, 30),
    ]),
    ("Midtown und Park", datetime(2026, 5, 16, 9, 0), [
        ("Top of the Rock", 40.7593, -73.9794, 30, 50),
        ("MoMA", 40.7614, -73.9776, 14, 30),
        ("Central Park Bethesda", 40.7739, -73.9712, 40, 80),
        ("Times Square", 40.7580, -73.9855, 16, 20),
    ]),
    ("High Line und Abreise", datetime(2026, 5, 17, 10, 0), [
        ("High Line", 40.7480, -74.0048, 26, 45),
    ]),
]


def main(ziel: Path) -> None:
    ziel.mkdir(parents=True, exist_ok=True)
    zufall = random.Random(11)
    nr = 0
    for ti, (_titel, start, orte) in enumerate(TAGE):
        zeit = start
        for ort_name, lat, lon, n, dauer in orte:
            i = 0
            schritt = max(1.0, dauer / n)
            while i < n:
                nr += 1
                zeit += timedelta(minutes=zufall.uniform(0.6 * schritt, 1.4 * schritt))
                hoch = zufall.random() < 0.22
                handy = zufall.random() < 0.7
                stunde = zeit.hour + zeit.minute / 60
                basis = szene(nr, stunde, hoch)
                marke, modell = ("Apple", "iPhone 15") if handy else ("Canon", "EOS R50")
                name = f"IMG_{1000 + nr}.jpg" if handy else f"_MG_{2000 + nr}.jpg"
                # Handy kennt den Ort (Streuung etwa 40 m), die Spiegelreflex hat kein GPS
                pos = (lat + zufall.gauss(0, 0.0003), lon + zufall.gauss(0, 0.0004)) if handy and zufall.random() > 0.04 else None
                if zufall.random() < 0.18:  # Serie aus 2 bis 4 Bildern
                    k = zufall.randint(2, 4)
                    weg = zufall.randrange(1, k)
                    for j in range(k):
                        speichere(variante(basis, j, unscharf=(j == weg and k > 2)), ziel / f"{name[:-4]}_{j}.jpg",
                                  zeit + timedelta(seconds=j * 3), marke=marke, modell=modell, gps=pos, quality=92)
                    i += k
                else:
                    speichere(basis, ziel / name, zeit, marke=marke, modell=modell, gps=pos, quality=92)
                    i += 1
                    if zufall.random() < 0.05:  # Messenger-Kopie: gleiche Datei
                        (ziel / f"WhatsApp {name}").write_bytes((ziel / name).read_bytes())
            zeit += timedelta(minutes=zufall.randint(20, 45))  # Weg zum nächsten Ort
        if ti in (1, 2):
            screenshot(ti).save(ziel / f"Screenshot_2026-05-{14 + ti}.png")
    for k, ti in enumerate((1, 3)):  # Panoramen (Seitenverhältnis 3:1), eigene Seite im Buch
        _t, start, orte = TAGE[ti]
        breit = szene(700 + k, 15 + k).crop((0, 330, 1600, 863))
        speichere(breit, ziel / f"PANO_{3000 + k}.jpg", start + timedelta(hours=3, minutes=7 + k), marke="Apple", modell="iPhone 15",
                  gps=(orte[0][1], orte[0][2]), quality=92)
    for j in range(5):  # Fotos ohne Aufnahmezeit (z. B. aus einem Chat gespeichert)
        szene(900 + j, 12).save(ziel / f"gespeichert_{j}.jpg", quality=85)
    print(f"{len(list(ziel.iterdir()))} Dateien in {ziel}")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
