"""Duplikate und Serien finden. Stufe 0: Datei-Hash und Differenzhash, keine KI."""

from __future__ import annotations

from dataclasses import dataclass, field

from .scanner import Foto


@dataclass
class Gruppe:
    art: str  # "exakt" oder "aehnlich"
    fotos: list[Foto] = field(default_factory=list)
    vorschlag: Foto | None = None  # Vorschlag: dieses Bild behalten


def hamming(a: int, b: int) -> int:
    return (a ^ b).bit_count()


def _bewertung(f: Foto, max_schaerfe: float) -> float:
    """Einfache Rangfolge für den Vorschlag: Schärfe zählt am meisten, dann Auflösung."""
    s = f.schaerfe / max_schaerfe if max_schaerfe > 0 else 0.0
    return 0.7 * s + 0.3 * min(f.megapixel / 12.0, 1.0)


def waehle_vorschlag(fotos: list[Foto]) -> Foto:
    maxs = max((f.schaerfe for f in fotos), default=0.0)
    return max(fotos, key=lambda f: (_bewertung(f, maxs), f.groesse_bytes))


def finde_exakte(fotos: list[Foto]) -> list[Gruppe]:
    nach_hash: dict[str, list[Foto]] = {}
    for f in fotos:
        if f.sha256 and not f.fehler:
            nach_hash.setdefault(f.sha256, []).append(f)
    gruppen = []
    for liste in nach_hash.values():
        if len(liste) > 1:
            g = Gruppe("exakt", sorted(liste, key=lambda f: f.pfad.lower()))
            g.vorschlag = g.fotos[0]
            gruppen.append(g)
    return gruppen


def finde_aehnliche(
    fotos: list[Foto],
    max_abstand: int = 6,
    fenster_sekunden: float = 900.0,
    max_spanne_sekunden: float | None = None,
    schon_exakt: set[str] | None = None,
) -> list[Gruppe]:
    """Serien: ähnlicher Differenzhash und kurzer zeitlicher Abstand.

    Eine Serie wächst nur, solange ihre gesamte Zeitspanne unter ``max_spanne_sekunden`` bleibt
    (Standard: doppeltes Fenster). So verketten sich ähnliche Motive eines langen Spaziergangs nicht
    zu einer Riesen-Serie.

    Exakte Duplikate (gleicher Datei-Hash) werden nur mit einem Vertreter geführt,
    damit Serien nicht durch Kopien aufgebläht werden.
    """
    if max_spanne_sekunden is None:
        max_spanne_sekunden = fenster_sekunden * 2
    kandidaten = [f for f in fotos if not f.fehler and f.breite > 0]
    gesehen: set[str] = set()
    vertreter: list[Foto] = []
    for f in sorted(kandidaten, key=lambda f: (f.zeit, f.name.lower())):
        if f.sha256 and f.sha256 in gesehen:
            continue
        if f.sha256:
            gesehen.add(f.sha256)
        vertreter.append(f)

    offene: list[list[Foto]] = []
    alle: list[list[Foto]] = []
    for f in vertreter:
        # Zu alte Serien schliessen
        offene = [c for c in offene if (f.zeit - c[-1].zeit).total_seconds() <= fenster_sekunden]
        ziel = None
        for c in offene:
            if (f.zeit - c[0].zeit).total_seconds() > max_spanne_sekunden:
                continue
            if any(hamming(f.dhash, m.dhash) <= max_abstand for m in c):
                ziel = c
                break
        if ziel is None:
            ziel = [f]
            offene.append(ziel)
            alle.append(ziel)
        else:
            ziel.append(f)

    gruppen = []
    for liste in alle:
        if len(liste) > 1:
            g = Gruppe("aehnlich", liste)
            g.vorschlag = waehle_vorschlag(liste)
            gruppen.append(g)
    gruppen.sort(key=lambda g: g.fotos[0].zeit)
    return gruppen
