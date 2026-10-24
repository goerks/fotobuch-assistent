"""Ereignisse bilden: Fotos nach Zeit sortieren und bei grossen Lücken oder Ortswechseln trennen."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime

from .scanner import Foto, entfernung_km

WOCHENTAGE = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]


@dataclass
class Ereignis:
    nummer: int
    fotos: list[Foto] = field(default_factory=list)

    @property
    def start(self) -> datetime:
        return self.fotos[0].zeit

    @property
    def ende(self) -> datetime:
        return self.fotos[-1].zeit

    @property
    def titel(self) -> str:
        s, e = self.start, self.ende
        tag = f"{WOCHENTAGE[s.weekday()]} {s:%d.%m.%Y}"
        if s.date() == e.date():
            return f"{tag} {s:%H:%M}–{e:%H:%M}"
        return f"{tag} {s:%H:%M} bis {WOCHENTAGE[e.weekday()]} {e:%d.%m.%Y %H:%M}"


def bilde_ereignisse(
    fotos: list[Foto],
    luecke_stunden: float = 6.0,
    max_km: float = 50.0,
) -> list[Ereignis]:
    """Sortiert nach Aufnahmezeit (dann Name) und trennt bei Zeitlücke oder grossem Ortssprung."""
    sortiert = sorted(fotos, key=lambda f: (f.zeit, f.name.lower()))
    ereignisse: list[Ereignis] = []
    aktuell: Ereignis | None = None
    letzte_gps = None
    for foto in sortiert:
        neu = aktuell is None
        if not neu:
            luecke = (foto.zeit - aktuell.fotos[-1].zeit).total_seconds() / 3600
            if luecke > luecke_stunden:
                neu = True
            elif foto.gps and letzte_gps and entfernung_km(foto.gps, letzte_gps) > max_km:
                neu = True
        if neu:
            aktuell = Ereignis(nummer=len(ereignisse) + 1)
            ereignisse.append(aktuell)
        aktuell.fotos.append(foto)
        if foto.gps:
            letzte_gps = foto.gps
    return ereignisse
