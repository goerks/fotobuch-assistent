"""Hilfsfunktionen: erzeugt synthetische Testbilder. Es werden nie echte Fotos benutzt."""

from datetime import datetime
from pathlib import Path

import numpy as np
import pytest
from PIL import Image, ImageFilter


def motiv(seed: int, size=(640, 480)) -> Image.Image:
    """Zufälliges, aber reproduzierbares Muster. Verschiedene Seeds ergeben verschiedene Bilder."""
    rng = np.random.default_rng(seed)
    klein = rng.integers(0, 255, (6, 8, 3), dtype=np.uint8)
    basis = np.asarray(Image.fromarray(klein).resize(size, Image.Resampling.BICUBIC), dtype=np.float32)
    # Feine Textur wie bei echten Fotos, damit Weichzeichnen die Schärfe messbar senkt
    textur = rng.normal(0, 25, (size[1], size[0], 1)).astype(np.float32)
    return Image.fromarray(np.clip(basis + textur, 0, 255).astype(np.uint8))


def speichere(bild: Image.Image, pfad: Path, zeit: datetime | None = None, orientierung: int | None = None,
              marke: str = "TestMarke", modell: str = "TestKamera", gps=None, **kw) -> Path:
    exif = Image.Exif()
    if zeit:
        sub = exif.get_ifd(0x8769)
        sub[36867] = zeit.strftime("%Y:%m:%d %H:%M:%S")
        exif[306] = zeit.strftime("%Y:%m:%d %H:%M:%S")
    if marke:
        exif[271] = marke
        exif[272] = modell
    if orientierung:
        exif[274] = orientierung
    if gps:
        def dms(x):
            x = abs(x)
            g = int(x)
            m = int((x - g) * 60)
            return (float(g), float(m), round((x - g - m / 60) * 3600, 3))
        ifd = exif.get_ifd(0x8825)
        ifd[1] = "N" if gps[0] >= 0 else "S"
        ifd[2] = dms(gps[0])
        ifd[3] = "E" if gps[1] >= 0 else "W"
        ifd[4] = dms(gps[1])
    bild.save(pfad, exif=exif, **kw)
    return pfad


@pytest.fixture
def ordner(tmp_path: Path) -> Path:
    return tmp_path


@pytest.fixture
def mache():
    return motiv, speichere, ImageFilter
