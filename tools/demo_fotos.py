"""Erzeugt einen realistischen Demo-Fotoordner (synthetische Bilder, keine echten Fotos).

Nachgebildet ist eine Städtereise über vier Tage: Serienbilder, Duplikate aus Messenger-Kopien,
Screenshots, Hoch- und Querformat, zwei Kameras, einige Fotos ohne Aufnahmezeit.
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


TAGE = [
    ("Ankunft und Altstadt", datetime(2026, 5, 14, 11, 40), 34, [(11.0, 3), (13.5, 2), (16.0, 4), (19.5, 1)]),
    ("Hafen und Strand", datetime(2026, 5, 15, 9, 10), 62, [(9.0, 4), (11.5, 3), (14.0, 6), (17.5, 4)]),
    ("Ausflug Bergdorf", datetime(2026, 5, 16, 8, 30), 71, [(8.5, 5), (11.0, 4), (13.0, 3), (16.0, 5)]),
    ("Markt und Abreise", datetime(2026, 5, 17, 9, 45), 28, [(9.8, 3), (12.0, 2)]),
]


def main(ziel: Path) -> None:
    ziel.mkdir(parents=True, exist_ok=True)
    zufall = random.Random(7)
    nr = 0
    for ti, (titel, start, n, _) in enumerate(TAGE):
        zeit = start
        i = 0
        while i < n:
            nr += 1
            zeit += timedelta(minutes=zufall.randint(2, 14))
            hoch = zufall.random() < 0.22
            handy = zufall.random() < 0.65
            stunde = zeit.hour + zeit.minute / 60
            basis = szene(nr, stunde, hoch)
            marke, modell = ("Apple", "iPhone 14") if handy else ("Canon", "EOS R50")
            name = f"IMG_{1000 + nr}.jpg" if handy else f"_MG_{2000 + nr}.jpg"
            art = zufall.random()
            if art < 0.20:  # Serie aus 2 bis 4 Bildern
                k = zufall.randint(2, 4)
                weg = zufall.randrange(k)
                for j in range(k):
                    speichere(variante(basis, j, unscharf=(j == weg and k > 2 and j > 0)), ziel / f"{name[:-4]}_{j}.jpg",
                              zeit + timedelta(seconds=j * 3), marke=marke, modell=modell, quality=92)
                i += k
            else:
                speichere(basis, ziel / name, zeit, marke=marke, modell=modell, quality=92)
                i += 1
                if zufall.random() < 0.06:  # Messenger-Kopie: gleiche Datei
                    (ziel / f"WhatsApp {name}").write_bytes((ziel / name).read_bytes())
        if ti in (1, 2):
            screenshot(ti).save(ziel / f"Screenshot_2026-05-{14 + ti}.png")
    for j in range(5):  # Fotos ohne Aufnahmezeit (z. B. aus Chat gespeichert)
        szene(900 + j, 12).save(ziel / f"gespeichert_{j}.jpg", quality=85)
    print(f"{len(list(ziel.iterdir()))} Dateien in {ziel}")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
