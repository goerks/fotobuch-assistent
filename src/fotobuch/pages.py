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
