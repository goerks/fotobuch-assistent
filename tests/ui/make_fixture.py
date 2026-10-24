"""Erzeugt einen Test-Fotoordner mit synthetischen Bildern (für die Oberflächentests)."""
import sys
from datetime import datetime, timedelta
from pathlib import Path

HIER = Path(__file__).resolve().parent
sys.path.insert(0, str(HIER.parent))
from conftest import motiv, speichere  # noqa: E402
from PIL import ImageFilter  # noqa: E402

ziel = Path(sys.argv[1])
ziel.mkdir(parents=True, exist_ok=True)
t0 = datetime(2026, 7, 3, 9, 0)
for i in range(12):
    speichere(motiv(i), ziel / f"a{i:02d}.jpg", t0 + timedelta(minutes=i * 40), gps=(40.7580 + i * 0.0001, -73.9855))
for i in range(6):
    speichere(motiv(100 + i), ziel / f"b{i}.jpg", t0 + timedelta(days=1, minutes=i * 40), gps=(40.7739, -73.9712))
basis = motiv(500)
speichere(basis, ziel / "s1.jpg", t0 + timedelta(days=1, hours=5))
speichere(basis.filter(ImageFilter.GaussianBlur(3)), ziel / "s2.jpg", t0 + timedelta(days=1, hours=5, minutes=1))
(ziel / "a03_kopie.jpg").write_bytes((ziel / "a03.jpg").read_bytes())
print("ok")
