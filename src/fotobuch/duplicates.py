"""Duplikate und Serien finden. Stufe 0: Datei-Hash und Differenzhash, keine KI."""

from __future__ import annotations

from dataclasses import dataclass, field

from .scanner import Foto


@dataclass
class Gruppe:
    art: str  # "exakt" oder "aehnlich"
    fotos: list[Foto] = field(default_factory=list)
    vorschlag: Foto | None = None  # Vorschlag: dieses Bild behalten


def schluessel(f: Foto) -> tuple:
    """Einheitliche Reihenfolge, damit exakte und ähnliche Gruppen denselben Vertreter wählen."""
    return (f.zeit, f.name.lower(), f.pfad.lower())


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
            g = Gruppe("exakt", sorted(liste, key=schluessel))
            g.vorschlag = g.fotos[0]
            gruppen.append(g)
    return gruppen


def finde_aehnliche(
    fotos: list[Foto],
    max_abstand: int = 6,
    fenster_sekunden: float = 900.0,
    max_spanne_sekunden: float | None = None,
    schon_exakt: set[str] | None = None,
    ort_von: dict[str, int] | None = None,
    fenster_ort_sekunden: float = 3600.0,
) -> list[Gruppe]:
    """Serien: ähnlicher Differenzhash und kurzer zeitlicher Abstand.

    Eine Serie wächst nur, solange ihre gesamte Zeitspanne unter ``max_spanne_sekunden`` bleibt
    (Standard: doppeltes Fenster). So verketten sich ähnliche Motive eines langen Spaziergangs nicht
    zu einer Riesen-Serie.

    Mit ``ort_von`` (Pfad -> Ort) gilt: Am selben Ort darf der zeitliche Abstand bis
    ``fenster_ort_sekunden`` betragen (man fotografiert dasselbe Motiv später nochmals), an
    verschiedenen Orten bilden Fotos nie eine Serie. Fehlt bei einem Foto der Ort, gilt das normale Fenster.

    Exakte Duplikate (gleicher Datei-Hash) werden nur mit einem Vertreter geführt,
    damit Serien nicht durch Kopien aufgebläht werden.
    """
    if max_spanne_sekunden is None:
        max_spanne_sekunden = fenster_sekunden * 2
    kandidaten = [f for f in fotos if not f.fehler and f.breite > 0]
    gesehen: set[str] = set()
    vertreter: list[Foto] = []
    for f in sorted(kandidaten, key=schluessel):
        if f.sha256 and f.sha256 in gesehen:
            continue
        if f.sha256:
            gesehen.add(f.sha256)
        vertreter.append(f)

    ort_von = ort_von or {}
    grosses_fenster = max(fenster_sekunden, fenster_ort_sekunden) if ort_von else fenster_sekunden

    def grenzen(a: Foto, b: Foto) -> tuple[float, float] | None:
        """(Zeitfenster, Spanne) für zwei Fotos oder None, wenn sie nie zusammengehören."""
        oa, ob = ort_von.get(a.pfad), ort_von.get(b.pfad)
        if oa is not None and ob is not None:
            if oa != ob:
                return None
            return fenster_ort_sekunden, max(max_spanne_sekunden, 2 * fenster_ort_sekunden)
        return fenster_sekunden, max_spanne_sekunden

    offene: list[list[Foto]] = []
    alle: list[list[Foto]] = []
    for f in vertreter:
        # Zu alte Serien schliessen
        offene = [c for c in offene if (f.zeit - c[-1].zeit).total_seconds() <= grosses_fenster]
        ziel = None
        for c in offene:
            for m in c:
                g = grenzen(f, m)
                if g is None:
                    continue
                fenster, spanne = g
                if (f.zeit - m.zeit).total_seconds() > fenster:
                    continue
                if (f.zeit - c[0].zeit).total_seconds() > spanne:
                    continue
                if hamming(f.dhash, m.dhash) <= max_abstand:
                    ziel = c
                    break
            if ziel is not None:
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
