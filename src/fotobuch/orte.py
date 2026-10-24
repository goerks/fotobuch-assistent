"""Orte bilden: Fotos, die an derselben Stelle entstanden sind, über GPS zusammenfassen.

Stufe 0, ohne KI und ohne Internet: Es gibt keine Ortsnamen, nur Koordinaten. Fotos ohne GPS
(zum Beispiel von einer Kamera ohne Ortungsmodul) werden dem Ort zugeordnet, an dem das zeitlich
nächste Foto mit GPS entstand, sofern es nicht zu lange her ist.
"""

from __future__ import annotations

import bisect
from dataclasses import dataclass, field

from .scanner import Foto, entfernung_km


@dataclass
class Ort:
    nummer: int
    fotos: list[Foto] = field(default_factory=list)
    lat: float = 0.0
    lon: float = 0.0
    mit_gps: int = 0

    @property
    def start(self):
        return min(f.zeit for f in self.fotos)

    @property
    def ende(self):
        return max(f.zeit for f in self.fotos)


def bilde_orte(
    fotos: list[Foto],
    radius_m: float = 200.0,
    zeit_toleranz_minuten: float = 30.0,
) -> tuple[list[Ort], dict[str, int]]:
    """Gibt die Orte und für jedes zugeordnete Foto (Pfad) den Index seines Ortes zurück.

    Ein Foto gehört zum nächsten bestehenden Ort, wenn dessen Mittelpunkt höchstens ``radius_m`` Meter
    entfernt ist, sonst beginnt ein neuer Ort. Kehrt man später an dieselbe Stelle zurück, landen die
    Fotos wieder im selben Ort.
    """
    mit = sorted((f for f in fotos if f.gps and not f.fehler), key=lambda f: (f.zeit, f.name.lower()))
    orte: list[Ort] = []
    summen: list[list[float]] = []
    zuordnung: dict[str, int] = {}
    for f in mit:
        bester, abstand = None, None
        for i, o in enumerate(orte):
            d = entfernung_km(f.gps, (o.lat, o.lon)) * 1000
            if d <= radius_m and (abstand is None or d < abstand):
                bester, abstand = i, d
        if bester is None:
            orte.append(Ort(nummer=len(orte) + 1, lat=f.gps[0], lon=f.gps[1]))
            summen.append([0.0, 0.0])
            bester = len(orte) - 1
        o = orte[bester]
        o.fotos.append(f)
        o.mit_gps += 1
        s = summen[bester]
        s[0] += f.gps[0]
        s[1] += f.gps[1]
        o.lat, o.lon = s[0] / o.mit_gps, s[1] / o.mit_gps
        zuordnung[f.pfad] = bester

    if mit:
        zeiten = [f.zeit for f in mit]
        for f in fotos:
            if f.pfad in zuordnung or f.fehler or f.screenshot_verdacht:
                continue
            i = bisect.bisect_left(zeiten, f.zeit)
            kand = [j for j in (i - 1, i) if 0 <= j < len(mit)]
            j = min(kand, key=lambda k: abs((mit[k].zeit - f.zeit).total_seconds()))
            if abs((mit[j].zeit - f.zeit).total_seconds()) <= zeit_toleranz_minuten * 60:
                idx = zuordnung[mit[j].pfad]
                orte[idx].fotos.append(f)
                zuordnung[f.pfad] = idx

    # Orte nach erstem Foto ordnen, Nummern neu vergeben
    reihenfolge = sorted(range(len(orte)), key=lambda i: orte[i].start)
    neu = {alt: n for n, alt in enumerate(reihenfolge)}
    sortiert = [orte[i] for i in reihenfolge]
    for n, o in enumerate(sortiert, 1):
        o.nummer = n
        o.fotos.sort(key=lambda f: (f.zeit, f.name.lower()))
    return sortiert, {p: neu[i] for p, i in zuordnung.items()}
