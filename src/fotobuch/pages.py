"""Seitenzähler: Wie viele Seiten ergeben die Fotos, und wie viele fehlen bis zur nächsten Seitenzahl?"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from importlib import resources
from pathlib import Path


@dataclass
class Profil:
    name: str
    verifiziert: bool
    seiten_min: int | None
    seiten_schritt: int | None
    seiten_max: int | None
    hinweis: str = ""

    def mit_vorgaben(self, seiten_min=None, seiten_schritt=None, seiten_max=None) -> "Profil":
        """Fehlende Profilwerte durch Vorgaben der Kommandozeile oder Annahmen ersetzen."""
        return Profil(
            name=self.name,
            verifiziert=self.verifiziert,
            seiten_min=seiten_min or self.seiten_min or 24,
            seiten_schritt=seiten_schritt or self.seiten_schritt or 2,
            seiten_max=seiten_max or self.seiten_max or 200,
            hinweis=self.hinweis,
        )


def lade_profil(name_oder_pfad: str) -> Profil:
    pfad = Path(name_oder_pfad)
    if pfad.suffix == ".json" and pfad.exists():
        daten = json.loads(pfad.read_text(encoding="utf-8"))
    else:
        daten = json.loads(
            resources.files("fotobuch").joinpath("profiles", f"{name_oder_pfad}.json").read_text(encoding="utf-8")
        )
    return Profil(
        name=daten.get("name", name_oder_pfad),
        verifiziert=bool(daten.get("verifiziert", False)),
        seiten_min=daten.get("seiten_min"),
        seiten_schritt=daten.get("seiten_schritt"),
        seiten_max=daten.get("seiten_max"),
        hinweis=daten.get("hinweis", ""),
    )


def erlaubte_seitenzahl(benoetigt: int, profil: Profil) -> int:
    """Kleinste erlaubte Seitenzahl, die mindestens ``benoetigt`` Seiten bietet."""
    mn, schritt = profil.seiten_min, profil.seiten_schritt
    if benoetigt <= mn:
        return mn
    stufen = math.ceil((benoetigt - mn) / schritt)
    return mn + stufen * schritt


def seitenplan(anzahl_fotos: int, fotos_pro_seite: float, profil: Profil) -> dict:
    """Rechnet für eine Fotozahl und eine durchschnittliche Fotozahl pro Seite.

    ``fehlende_fotos``: so viele Fotos braucht es, um die gewählte Seitenzahl bei diesem Schnitt zu füllen.
    ``ueberzaehlige_fotos``: so viele Fotos müssten weg, um eine Seitenstufe zu sparen.
    """
    fps = max(fotos_pro_seite, 0.1)
    benoetigt = math.ceil(anzahl_fotos / fps)
    buch = erlaubte_seitenzahl(max(benoetigt, 1), profil)
    fehlend = max(0, math.ceil(buch * fps) - anzahl_fotos)
    vorherige = buch - profil.seiten_schritt
    ueberzaehlig = 0
    if vorherige >= profil.seiten_min:
        ueberzaehlig = max(0, anzahl_fotos - math.floor(vorherige * fps))
    return {
        "fotos": anzahl_fotos,
        "fotos_pro_seite": fps,
        "seiten_benoetigt": benoetigt,
        "buch_seiten": buch,
        "ueber_maximum": buch > profil.seiten_max,
        "fehlende_fotos": fehlend,
        "ueberzaehlige_fotos_fuer_kleinere_stufe": ueberzaehlig,
    }


# ---------------------------------------------------------------------------
# Ereignisbewusster Seitenplan (für die App)
# ---------------------------------------------------------------------------


def verteile(n: int, fps: int) -> list[int]:
    """Verteilt ``n`` Fotos möglichst gleichmässig auf die nötigen Seiten (je höchstens ``fps``)."""
    if n <= 0:
        return []
    seiten = math.ceil(n / fps)
    basis, rest = divmod(n, seiten)
    return [basis + 1 if i < rest else basis for i in range(seiten)]


def _gruppen(counts: list[int], getrennt: bool) -> list[int]:
    return [c for c in counts if c > 0] if getrennt else ([sum(counts)] if sum(counts) > 0 else [])


def seiten_benoetigt(counts: list[int], fps: int, getrennt: bool = True) -> int:
    """Seiten, wenn jedes Ereignis auf einer neuen Seite beginnt (``getrennt``) oder lückenlos gefüllt wird."""
    return sum(math.ceil(c / fps) for c in _gruppen(counts, getrennt))


def kuerzungsplan(counts: list[int], fps: int, getrennt: bool, seiten_sparen: int) -> dict[int, int]:
    """Günstigste Verteilung, um ``seiten_sparen`` Seiten einzusparen: Ereignis-Index -> Fotos weglassen.

    Eine Seite lässt sich je Ereignis zuerst über die Restfotos der letzten Seite sparen (billig),
    danach über je ``fps`` Fotos. Ist ``getrennt`` falsch, gilt das ganze Buch als ein Ereignis.
    """
    if seiten_sparen <= 0:
        return {}
    if getrennt:
        indizes = [i for i, c in enumerate(counts) if c > 0]
        werte = [counts[i] for i in indizes]
    else:
        indizes = [0]
        werte = [sum(counts)]
    kandidaten: list[tuple[int, int]] = []
    for idx, c in zip(indizes, werte):
        seiten = math.ceil(c / fps)
        rest = c % fps
        kandidaten.append((rest if rest else fps, idx))
        kandidaten.extend((fps, idx) for _ in range(seiten - 1))
    kandidaten.sort()
    plan: dict[int, int] = {}
    for kosten, idx in kandidaten[:seiten_sparen]:
        plan[idx] = plan.get(idx, 0) + kosten
    return plan


def seitenplan_ereignisse(counts: list[int], fps: int, profil: Profil, getrennt: bool = True) -> dict:
    """Seitenzähler für die App.

    ``fehlende_fotos``: So viele Fotos passen noch dazu, ohne dass die gewählte Buchseitenzahl überschritten wird.
    ``weglassen_fuer_kleinere_stufe``: So viele Fotos müssten weg, um eine Seitenstufe zu sparen.
    """
    fps = max(1, int(fps))
    total = sum(counts)
    benoetigt = seiten_benoetigt(counts, fps, getrennt)
    buch = erlaubte_seitenzahl(max(benoetigt, 1), profil)
    kleinere = buch - profil.seiten_schritt
    weglassen = None
    plan: dict[int, int] = {}
    if kleinere >= profil.seiten_min and benoetigt > kleinere:
        plan = kuerzungsplan(counts, fps, getrennt, benoetigt - kleinere)
        weglassen = sum(plan.values())
    return {
        "fotos": total,
        "fotos_pro_seite": fps,
        "ereignisse_getrennt": getrennt,
        "seiten_benoetigt": benoetigt,
        "buch_seiten": buch,
        "leere_seiten": max(0, buch - benoetigt),
        "ueber_maximum": buch > profil.seiten_max,
        "fehlende_fotos": max(0, buch * fps - total),
        "kleinere_stufe": kleinere if weglassen is not None else None,
        "weglassen_fuer_kleinere_stufe": weglassen,
        "kuerzungsplan": {str(k): v for k, v in plan.items()},
        "seiten_je_ereignis": [math.ceil(c / fps) if c > 0 else 0 for c in counts],
    }


def seitenzuordnung(counts: list[int], fps: int, getrennt: bool = True) -> list[tuple[int, list[int]]]:
    """Liste von Seiten: (Ereignis-Index oder -1 bei lückenloser Füllung, Fotos je Seite)."""
    fps = max(1, int(fps))
    seiten: list[tuple[int, list[int]]] = []
    if getrennt:
        for i, c in enumerate(counts):
            for n in verteile(c, fps):
                seiten.append((i, [n]))
    else:
        for n in verteile(sum(counts), fps):
            seiten.append((-1, [n]))
    return seiten
