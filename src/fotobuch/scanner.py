"""Einlesen von Fotos: Aufnahmezeit, GPS, Grösse, Hashes, Schärfe. Rein lokal, ohne KI."""

from __future__ import annotations

import hashlib
import io
import math
import os
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Callable, Iterable

import numpy as np
from PIL import Image, ImageOps

try:  # HEIC (iPhone)
    import pillow_heif

    pillow_heif.register_heif_opener()
    HEIC_OK = True
except Exception:  # pragma: no cover - hängt von der Installation ab
    HEIC_OK = False

try:  # RAW-Vorschaubilder, optional
    import rawpy

    RAW_OK = True
except Exception:  # pragma: no cover
    RAW_OK = False

JPEG_EXT = {".jpg", ".jpeg", ".jpe"}
HEIC_EXT = {".heic", ".heif"}
PNG_EXT = {".png"}
OTHER_EXT = {".tif", ".tiff", ".webp", ".bmp"}
RAW_EXT = {".dng", ".cr2", ".cr3", ".nef", ".arw", ".orf", ".rw2", ".raf", ".pef", ".srw"}
ALL_EXT = JPEG_EXT | HEIC_EXT | PNG_EXT | OTHER_EXT | RAW_EXT

SCREENSHOT_HINTS = ("screenshot", "bildschirmfoto", "bildschirmaufnahme", "screen shot", "capture")
ANALYSE_KANTE = 512  # Längste Kante der Analysebilder in Pixeln


@dataclass
class Foto:
    pfad: str
    name: str
    endung: str
    groesse_bytes: int
    zeit: datetime
    zeit_quelle: str  # "exif" oder "datei"
    breite: int = 0
    hoehe: int = 0
    kamera: str = ""
    gps: tuple[float, float] | None = None
    sha256: str = ""
    dhash: int = 0
    schaerfe: float = 0.0
    helligkeit: float = 0.0
    ist_raw: bool = False
    screenshot_verdacht: bool = False
    fehler: str = ""
    hinweise: list[str] = field(default_factory=list)

    @property
    def format(self) -> str:
        if self.breite == 0 or self.hoehe == 0:
            return "unbekannt"
        if self.breite > self.hoehe * 1.02:
            return "quer"
        if self.hoehe > self.breite * 1.02:
            return "hoch"
        return "quadrat"

    @property
    def megapixel(self) -> float:
        return self.breite * self.hoehe / 1e6


def finde_fotos(ordner: str | Path) -> list[Path]:
    """Alle unterstützten Bilddateien unterhalb von ``ordner``, sortiert nach Pfad."""
    gefunden: list[Path] = []
    for wurzel, _dirs, dateien in os.walk(ordner):
        for d in dateien:
            if Path(d).suffix.lower() in ALL_EXT and not d.startswith("._"):
                gefunden.append(Path(wurzel) / d)
    gefunden.sort(key=lambda p: str(p).lower())
    return gefunden


def _sha256(pfad: Path) -> str:
    h = hashlib.sha256()
    with open(pfad, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def _ratio(wert) -> float:
    try:
        return float(wert)
    except Exception:
        n, d = wert
        return float(n) / float(d) if d else 0.0


def _gps_aus_exif(exif) -> tuple[float, float] | None:
    try:
        gps = exif.get_ifd(0x8825)
        if not gps or 2 not in gps or 4 not in gps:
            return None

        def dez(teile, ref):
            grad = _ratio(teile[0]) + _ratio(teile[1]) / 60 + _ratio(teile[2]) / 3600
            return -grad if ref in ("S", "W") else grad

        lat = dez(gps[2], gps.get(1, "N"))
        lon = dez(gps[4], gps.get(3, "E"))
        if lat == 0 and lon == 0:
            return None
        return (lat, lon)
    except Exception:
        return None


def _zeit_aus_exif(exif) -> datetime | None:
    kandidaten = []
    try:
        kandidaten.append(exif.get_ifd(0x8769).get(36867))  # DateTimeOriginal
        kandidaten.append(exif.get_ifd(0x8769).get(36868))  # DateTimeDigitized
    except Exception:
        pass
    kandidaten.append(exif.get(306))  # DateTime
    for k in kandidaten:
        if not k or not isinstance(k, str):
            continue
        for fmt in ("%Y:%m:%d %H:%M:%S", "%Y-%m-%d %H:%M:%S"):
            try:
                zeit = datetime.strptime(k.strip()[:19], fmt)
                if zeit.year >= 1990:
                    return zeit
            except ValueError:
                continue
    return None


def _oeffne(pfad: Path) -> tuple[Image.Image, bool]:
    """Öffnet ein Bild als PIL-Image. Bei RAW wird die eingebettete Vorschau benutzt."""
    if pfad.suffix.lower() in RAW_EXT:
        if not RAW_OK:
            raise RuntimeError("RAW-Unterstützung nicht installiert (rawpy fehlt)")
        with rawpy.imread(str(pfad)) as raw:
            thumb = raw.extract_thumb()
        if thumb.format == rawpy.ThumbFormat.JPEG:
            return Image.open(io.BytesIO(thumb.data)), True
        arr = thumb.data
        return Image.fromarray(arr), True
    return Image.open(pfad), False


def _schaerfe(grau: np.ndarray) -> float:
    """Varianz des Laplace-Filters. Höher heisst schärfer. Nur innerhalb eines Laufs vergleichbar."""
    if grau.shape[0] < 3 or grau.shape[1] < 3:
        return 0.0
    c = grau[1:-1, 1:-1]
    lap = 4 * c - grau[:-2, 1:-1] - grau[2:, 1:-1] - grau[1:-1, :-2] - grau[1:-1, 2:]
    return float(lap.var())


def _dhash(grau_klein: np.ndarray) -> int:
    """64-Bit-Differenzhash aus einem 9x8-Graubild."""
    diff = grau_klein[:, 1:] > grau_klein[:, :-1]
    wert = 0
    for bit in diff.flatten():
        wert = (wert << 1) | int(bit)
    return wert


def analysiere_foto(pfad: Path, mit_hash: bool = True) -> Foto:
    st = pfad.stat()
    mtime = datetime.fromtimestamp(st.st_mtime)
    foto = Foto(
        pfad=str(pfad),
        name=pfad.name,
        endung=pfad.suffix.lower(),
        groesse_bytes=st.st_size,
        zeit=mtime,
        zeit_quelle="datei",
        ist_raw=pfad.suffix.lower() in RAW_EXT,
    )
    nameklein = pfad.stem.lower()
    try:
        if mit_hash:
            foto.sha256 = _sha256(pfad)
        img, _raw = _oeffne(pfad)
        with img:
            exif = img.getexif()
            zeit = _zeit_aus_exif(exif) if exif else None
            if zeit:
                foto.zeit, foto.zeit_quelle = zeit, "exif"
            foto.gps = _gps_aus_exif(exif) if exif else None
            marke = str(exif.get(271, "")).strip() if exif else ""
            modell = str(exif.get(272, "")).strip() if exif else ""
            foto.kamera = (modell if modell.startswith(marke) else f"{marke} {modell}").strip()

            breite, hoehe = img.size  # Echte Grösse, bevor JPEG-Draft verkleinert
            if exif and exif.get(274, 1) in (5, 6, 7, 8):  # Hochformat per EXIF-Drehung
                breite, hoehe = hoehe, breite
            foto.breite, foto.hoehe = breite, hoehe
            if img.format == "JPEG":
                img.draft("RGB", (ANALYSE_KANTE * 2, ANALYSE_KANTE * 2))
            img = ImageOps.exif_transpose(img)
            if _raw:
                foto.hinweise.append("RAW: Analyse über eingebettete Vorschau")
            img.thumbnail((ANALYSE_KANTE, ANALYSE_KANTE))
            grau = np.asarray(img.convert("L"), dtype=np.float32)
            foto.schaerfe = _schaerfe(grau)
            foto.helligkeit = float(grau.mean())
            klein = np.asarray(
                Image.fromarray(grau.astype(np.uint8)).resize((9, 8), Image.Resampling.BILINEAR),
                dtype=np.int16,
            )
            foto.dhash = _dhash(klein)
    except Exception as e:  # Eine kaputte Datei soll den Lauf nicht abbrechen
        foto.fehler = f"{type(e).__name__}: {e}"

    ist_png = foto.endung in PNG_EXT
    if any(h in nameklein for h in SCREENSHOT_HINTS) or (ist_png and not foto.kamera):
        foto.screenshot_verdacht = True
    if foto.zeit_quelle == "datei":
        foto.hinweise.append("Keine Aufnahmezeit in den Metadaten, Dateidatum benutzt")
    return foto


def scanne(
    pfade: Iterable[Path],
    threads: int | None = None,
    fortschritt: Callable[[int, int], None] | None = None,
) -> list[Foto]:
    pfade = list(pfade)
    n = len(pfade)
    threads = threads or max(2, min(8, (os.cpu_count() or 2)))
    ergebnis: list[Foto] = []
    with ThreadPoolExecutor(max_workers=threads) as ex:
        for i, foto in enumerate(ex.map(analysiere_foto, pfade), 1):
            ergebnis.append(foto)
            if fortschritt:
                fortschritt(i, n)
    return ergebnis


def entfernung_km(a: tuple[float, float], b: tuple[float, float]) -> float:
    r = 6371.0
    p1, p2 = math.radians(a[0]), math.radians(b[0])
    dp = p2 - p1
    dl = math.radians(b[1] - a[1])
    h = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * r * math.asin(math.sqrt(h))
