"""Projektzustand der App: Fotos, Gruppen, Entscheidungen, Seitenplan, Speicherung, Export.

Ein Projekt gehört zu einem Fotoordner. Alles wird im Unterordner ``.fotobuch`` des Fotoordners abgelegt
(Zustand und Vorschaubilder). Die Originalfotos werden nie verändert.
"""

from __future__ import annotations

import csv
import hashlib
import io
import json
import os
import re
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict
from datetime import datetime
from pathlib import Path
from typing import Callable

import numpy as np
from PIL import Image, ImageOps

from . import __version__
from .duplicates import schluessel
from .pages import Profil, lade_profil, seitenplan_ereignisse, seitenzuordnung
from .report import Optionen, baue_ergebnis, Ergebnis
from .scanner import (
    EIGENE_ORDNER,
    Foto,
    _oeffne,
    analysiere_foto,
    finde_fotos,
)

ZUSTANDSDATEI = "projekt.json"
FORMAT_VERSION = 1
PROFILE = ["allgemein", "cewe-pixum", "ifolor"]

STANDARD = {
    "luecke_stunden": 6.0,
    "max_km": 50.0,
    "aehnlich_abstand": 6,
    "serie_minuten": 15.0,
    "fotos_pro_seite": 3,
    "profil": "allgemein",
    "seiten_min": None,
    "seiten_schritt": None,
    "seiten_max": None,
    "ereignisse_getrennt": True,
}

GRENZEN = {
    "luecke_stunden": (0.5, 72.0),
    "max_km": (1.0, 5000.0),
    "aehnlich_abstand": (0, 16),
    "serie_minuten": (1.0, 120.0),
    "fotos_pro_seite": (1, 8),
}


class ProjektFehler(Exception):
    """Fehler mit einer für Nutzer lesbaren Meldung."""


def foto_id(rel: str) -> str:
    return hashlib.sha1(rel.replace("\\", "/").lower().encode("utf-8")).hexdigest()[:12]


def _sauber(name: str) -> str:
    name = re.sub(r'[<>:"/\\|?*\x00-\x1f]', "_", name).strip(" .")
    return name or "_"


def _foto_zu_dict(f: Foto, rel: str) -> dict:
    d = asdict(f)
    d["pfad"] = rel
    d["zeit"] = f.zeit.isoformat()
    d["dhash"] = format(f.dhash, "016x")
    d["gps"] = list(f.gps) if f.gps else None
    return d


def _foto_aus_dict(d: dict, wurzel: Path) -> Foto:
    d = dict(d)
    rel = d.pop("pfad")
    d["pfad"] = str(wurzel / rel)
    d["zeit"] = datetime.fromisoformat(d["zeit"])
    d["dhash"] = int(d["dhash"], 16)
    d["gps"] = tuple(d["gps"]) if d.get("gps") else None
    return Foto(**d)


def _score(f: Foto, maxs: float) -> float:
    s = min(f.schaerfe / maxs, 1.0) if maxs > 0 else 0.0
    return 0.7 * s + 0.3 * min(f.megapixel / 12.0, 1.0)


class Projekt:
    def __init__(self, ordner: str | Path):
        self.ordner = Path(ordner).resolve()
        if not self.ordner.is_dir():
            raise ProjektFehler(f"Der Ordner existiert nicht: {self.ordner}")
        self.daten = self.ordner / ".fotobuch"
        self.lock = threading.RLock()
        self.einstellungen: dict = dict(STANDARD)
        self.entscheidungen: dict[str, str] = {}
        self.ereignis_namen: dict[str, str] = {}
        self.schritt: int = 1
        self.cache: dict[str, dict] = {}  # rel -> {"mtime_ns", "groesse", "foto"}
        self.fotos: dict[str, Foto] = {}
        self.rel_von_id: dict[str, str] = {}
        self.ergebnis: Ergebnis | None = None
        self.analysiert_am: str | None = None
        self.analyse_dauer: float | None = None
        self.profil_vorgaben: Profil | None = None
        self._laden()
        if self.cache:
            self._fotos_aus_cache()
            self.neu_gruppieren()

    # ------------------------------------------------------------------ Speicherung

    def _laden(self) -> None:
        pfad = self.daten / ZUSTANDSDATEI
        if not pfad.exists():
            return
        try:
            d = json.loads(pfad.read_text(encoding="utf-8"))
            if d.get("version") != FORMAT_VERSION:
                return
        except Exception:
            # Defekte Datei nicht löschen, sondern beiseite legen
            try:
                pfad.rename(pfad.with_suffix(".defekt"))
            except OSError:
                pass
            return
        self.einstellungen.update({k: v for k, v in d.get("einstellungen", {}).items() if k in STANDARD})
        self.entscheidungen = d.get("entscheidungen", {})
        self.ereignis_namen = d.get("ereignis_namen", {})
        self.schritt = int(d.get("schritt", 1))
        self.cache = d.get("cache", {})
        self.analysiert_am = d.get("analysiert_am")
        self.analyse_dauer = d.get("analyse_dauer")

    def speichern(self) -> None:
        with self.lock:
            self.daten.mkdir(parents=True, exist_ok=True)
            d = {
                "version": FORMAT_VERSION,
                "programmversion": __version__,
                "einstellungen": self.einstellungen,
                "entscheidungen": self.entscheidungen,
                "ereignis_namen": self.ereignis_namen,
                "schritt": self.schritt,
                "cache": self.cache,
                "analysiert_am": self.analysiert_am,
                "analyse_dauer": self.analyse_dauer,
            }
            ziel = self.daten / ZUSTANDSDATEI
            tmp = ziel.with_suffix(".tmp")
            tmp.write_text(json.dumps(d, ensure_ascii=False), encoding="utf-8")
            os.replace(tmp, ziel)

    # ------------------------------------------------------------------ Analyse

    def _fotos_aus_cache(self) -> None:
        self.fotos = {}
        self.rel_von_id = {}
        for rel, eintrag in self.cache.items():
            f = _foto_aus_dict(eintrag["foto"], self.ordner)
            fid = foto_id(rel)
            self.fotos[fid] = f
            self.rel_von_id[fid] = rel

    def analysieren(self, fortschritt: Callable[[str, int, int], None] | None = None, threads: int | None = None) -> None:
        """Liest neue und geänderte Fotos ein. Unveränderte kommen aus dem Zwischenspeicher."""
        t0 = time.perf_counter()
        pfade = finde_fotos(self.ordner)
        neu_cache: dict[str, dict] = {}
        zu_lesen: list[tuple[str, Path, int, int]] = []
        for p in pfade:
            rel = p.relative_to(self.ordner).as_posix()
            st = p.stat()
            alt = self.cache.get(rel)
            if alt and alt["mtime_ns"] == st.st_mtime_ns and alt["groesse"] == st.st_size:
                neu_cache[rel] = alt
            else:
                zu_lesen.append((rel, p, st.st_mtime_ns, st.st_size))
        n = len(zu_lesen)
        if fortschritt:
            fortschritt("einlesen", 0, n)
        threads = threads or max(2, min(8, (os.cpu_count() or 2)))

        def arbeit(x):
            rel, p, mt, gr = x
            return rel, mt, gr, analysiere_foto(p)

        with ThreadPoolExecutor(max_workers=threads) as ex:
            for i, (rel, mt, gr, foto) in enumerate(ex.map(arbeit, zu_lesen), 1):
                neu_cache[rel] = {"mtime_ns": mt, "groesse": gr, "foto": _foto_zu_dict(foto, rel)}
                if fortschritt:
                    fortschritt("einlesen", i, n)
        with self.lock:
            self.cache = neu_cache
            self._fotos_aus_cache()
            self.analysiert_am = datetime.now().isoformat(timespec="seconds")
            self.analyse_dauer = round(time.perf_counter() - t0, 2)
            # Entscheidungen zu verschwundenen Fotos verwerfen
            self.entscheidungen = {k: v for k, v in self.entscheidungen.items() if k in self.fotos}
            self.neu_gruppieren()
            self.speichern()

    # ------------------------------------------------------------------ Gruppierung

    def profil(self) -> Profil:
        e = self.einstellungen
        try:
            basis = lade_profil(e["profil"])
        except Exception:
            basis = lade_profil("allgemein")
        return basis.mit_vorgaben(e.get("seiten_min"), e.get("seiten_schritt"), e.get("seiten_max"))

    def neu_gruppieren(self) -> None:
        with self.lock:
            e = self.einstellungen
            opt = Optionen(
                luecke_stunden=float(e["luecke_stunden"]),
                max_km=float(e["max_km"]),
                aehnlich_abstand=int(e["aehnlich_abstand"]),
                serie_minuten=float(e["serie_minuten"]),
                fotos_pro_seite=float(e["fotos_pro_seite"]),
                profil=self.profil(),
            )
            fotos = sorted(self.fotos.values(), key=schluessel)
            self.ergebnis = baue_ergebnis(self.ordner, fotos, opt)
            self._id_von_pfad = {f.pfad: fid for fid, f in self.fotos.items()}
            # Stabile Reihenfolge der Serien
            self._ereignis_von_id: dict[str, int] = {}
            for i, ev in enumerate(self.ergebnis.ereignisse):
                for f in ev.fotos:
                    self._ereignis_von_id[self._id_von_pfad[f.pfad]] = i
            vals = [f.schaerfe for f in fotos if not f.fehler and f.schaerfe > 0]
            self._maxs = float(np.percentile(vals, 90)) if vals else 0.0

    def _fid(self, f: Foto) -> str:
        return self._id_von_pfad[f.pfad]

    # ------------------------------------------------------------------ Entscheidungen

    def _auto(self, f: Foto) -> tuple[bool, str]:
        """Automatische Einstufung: (behalten, Grund)."""
        erg = self.ergebnis
        if f.fehler:
            return False, "Nicht lesbar"
        if f.pfad in erg.zu_entfernen_exakt:
            return False, "Exaktes Duplikat"
        if f.screenshot_verdacht:
            return False, "Screenshot"
        if f.pfad in erg.zu_entfernen_serien:
            return False, "Serie, anderes Foto vorgeschlagen"
        return True, ""

    def einstufung(self, fid: str) -> dict:
        f = self.fotos[fid]
        auto_behalten, grund = self._auto(f)
        eig = self.entscheidungen.get(fid)
        if eig == "behalten":
            return {"behalten": True, "grund": "Von dir behalten", "eigene": True}
        if eig == "entfernen":
            return {"behalten": False, "grund": "Von dir entfernt", "eigene": True}
        return {"behalten": auto_behalten, "grund": grund, "eigene": False}

    def setze_foto(self, fid: str, aktion: str) -> None:
        with self.lock:
            if fid not in self.fotos:
                raise ProjektFehler("Unbekanntes Foto")
            if aktion == "auto":
                self.entscheidungen.pop(fid, None)
            elif aktion in ("behalten", "entfernen"):
                self.entscheidungen[fid] = aktion
            else:
                raise ProjektFehler("Unbekannte Aktion")
            self.speichern()

    def serie_entscheiden(self, nr: int, waehle) -> None:
        """``waehle``: Foto-ID, Liste von IDs, ``"vorschlag"`` (zurück zum Vorschlag) oder ``"keines"``."""
        with self.lock:
            serien = self.ergebnis.serien
            if not 0 <= nr < len(serien):
                raise ProjektFehler("Unbekannte Serie")
            g = serien[nr]
            ids = [self._fid(f) for f in g.fotos]
            if waehle == "vorschlag":
                for i in ids:
                    self.entscheidungen.pop(i, None)
            elif waehle == "keines":
                for i in ids:
                    self.entscheidungen[i] = "entfernen"
            else:
                gewaehlt = [waehle] if isinstance(waehle, str) else list(waehle)
                if not gewaehlt or any(x not in ids for x in gewaehlt):
                    raise ProjektFehler("Die Auswahl gehört nicht zu dieser Serie")
                for i in ids:
                    self.entscheidungen[i] = "behalten" if i in gewaehlt else "entfernen"
            self.speichern()

    def alle_serien_auf_vorschlag(self) -> None:
        with self.lock:
            for g in self.ergebnis.serien:
                for f in g.fotos:
                    self.entscheidungen.pop(self._fid(f), None)
            self.speichern()

    def alle_entscheidungen_zuruecksetzen(self) -> None:
        with self.lock:
            self.entscheidungen = {}
            self.speichern()

    def ereignis_umbenennen(self, key: str, name: str) -> None:
        with self.lock:
            name = (name or "").strip()[:80]
            if name:
                self.ereignis_namen[key] = name
            else:
                self.ereignis_namen.pop(key, None)
            self.speichern()

    def setze_einstellungen(self, neu: dict) -> None:
        with self.lock:
            for k, v in neu.items():
                if k not in STANDARD:
                    raise ProjektFehler(f"Unbekannte Einstellung: {k}")
                if k in GRENZEN:
                    lo, hi = GRENZEN[k]
                    try:
                        v = type(STANDARD[k])(v)
                    except (TypeError, ValueError):
                        raise ProjektFehler(f"Ungültiger Wert für {k}")
                    if not lo <= v <= hi:
                        raise ProjektFehler(f"{k} muss zwischen {lo} und {hi} liegen")
                elif k == "profil":
                    if v not in PROFILE:
                        raise ProjektFehler("Unbekanntes Profil")
                elif k == "ereignisse_getrennt":
                    v = bool(v)
                elif k in ("seiten_min", "seiten_schritt", "seiten_max"):
                    if v in (None, ""):
                        v = None
                    else:
                        try:
                            v = int(v)
                        except (TypeError, ValueError):
                            raise ProjektFehler(f"Ungültiger Wert für {k}")
                        if v < 1 or v > 1000:
                            raise ProjektFehler(f"{k} ist unrealistisch")
                self.einstellungen[k] = v
            if neu.get("profil"):
                # Beim Profilwechsel eigene Seitenregeln zurücksetzen
                for k in ("seiten_min", "seiten_schritt", "seiten_max"):
                    if k not in neu:
                        self.einstellungen[k] = None
            self.neu_gruppieren()
            self.speichern()

    def setze_schritt(self, schritt: int) -> None:
        with self.lock:
            self.schritt = max(1, min(5, int(schritt)))
            self.speichern()

    # ------------------------------------------------------------------ Auswertung

    def _ereignis_zahlen(self) -> list[tuple[list[str], list[str]]]:
        """Je Ereignis: (alle IDs, IDs im Buch)."""
        res = []
        for ev in self.ergebnis.ereignisse:
            ids = [self._fid(f) for f in ev.fotos]
            buch = [i for i in ids if self.einstufung(i)["behalten"]]
            res.append((ids, buch))
        return res

    def im_buch(self) -> list[str]:
        out: list[str] = []
        for _ids, buch in self._ereignis_zahlen():
            out.extend(buch)
        return out

    def seitenplan(self) -> dict:
        zahlen = [len(b) for _a, b in self._ereignis_zahlen()]
        return seitenplan_ereignisse(
            zahlen, int(self.einstellungen["fotos_pro_seite"]), self.profil(), bool(self.einstellungen["ereignisse_getrennt"])
        )

    def vorschlaege(self, plan: dict | None = None) -> dict:
        """Konkrete Fotos zum Auffüllen oder Kürzen, passend zum Seitenzähler."""
        plan = plan or self.seitenplan()
        fps = int(self.einstellungen["fotos_pro_seite"])
        getrennt = bool(self.einstellungen["ereignisse_getrennt"])
        zahlen = self._ereignis_zahlen()
        maxs = self._maxs

        # Auffüllen: Serien-Reste zurückholen, ohne dass eine neue Seite entsteht
        auf: list[str] = []
        fehlt = plan["fehlende_fotos"]
        if fehlt > 0:
            if getrennt:
                frei = [max(0, -(-len(b) // fps) * fps - len(b)) if len(b) else 0 for _a, b in zahlen]
            else:
                frei = [fehlt]
            kandidaten = []
            for ei, (ids, buch) in enumerate(zahlen):
                for i in ids:
                    einst = self.einstufung(i)
                    if einst["behalten"] or einst["eigene"]:
                        continue
                    if not einst["grund"].startswith("Serie"):
                        continue
                    kandidaten.append((-_score(self.fotos[i], maxs), i, ei if getrennt else 0))
            kandidaten.sort()
            for _s, i, ei in kandidaten:
                if len(auf) >= fehlt:
                    break
                if frei[ei] > 0:
                    frei[ei] -= 1
                    auf.append(i)

        # Kürzen: schwächste Fotos genau dort, wo dadurch Seiten wegfallen
        kuerzen: list[str] = []
        if plan["weglassen_fuer_kleinere_stufe"]:
            for key, anzahl in plan["kuerzungsplan"].items():
                ei = int(key)
                if getrennt:
                    buch = zahlen[ei][1]
                else:
                    buch = [i for _a, b in zahlen for i in b]
                # Eigene Entscheidungen zum Behalten sind geschützt
                frei_zum_entfernen = [i for i in buch if self.entscheidungen.get(i) != "behalten"]
                frei_zum_entfernen.sort(key=lambda i: _score(self.fotos[i], maxs))
                kuerzen.extend(frei_zum_entfernen[:anzahl])
            if len(kuerzen) != plan["weglassen_fuer_kleinere_stufe"]:
                kuerzen = []  # Nicht exakt machbar, lieber nichts vorschlagen
        return {"auffuellen": auf, "kuerzen": kuerzen}

    def vorschlag_anwenden(self, art: str, ids: list[str] | None = None) -> int:
        with self.lock:
            v = self.vorschlaege()
            if art not in v:
                raise ProjektFehler("Unbekannter Vorschlag")
            erlaubt = set(v[art])
            ziel = [i for i in (ids if ids is not None else v[art]) if i in erlaubt]
            for i in ziel:
                self.entscheidungen[i] = "behalten" if art == "auffuellen" else "entfernen"
            self.speichern()
            return len(ziel)

    # ------------------------------------------------------------------ Ansicht

    def ansicht(self) -> dict:
        with self.lock:
            if self.ergebnis is None or not self.fotos:
                return {
                    "programmversion": __version__,
                    "ordner": str(self.ordner),
                    "name": self.ordner.name,
                    "schritt": 1,
                    "analysiert": False,
                    "leer": self.analysiert_am is not None,
                    "einstellungen": self.einstellungen,
                }
            erg = self.ergebnis
            fotos = list(self.fotos.values())
            lesbar = [f for f in fotos if not f.fehler]
            endungen: dict[str, int] = {}
            for f in fotos:
                endungen[f.endung] = endungen.get(f.endung, 0) + 1
            einst = {fid: self.einstufung(fid) for fid in self.fotos}
            im_buch = sum(1 for v in einst.values() if v["behalten"])
            seitenplan = self.seitenplan()
            profil = self.profil()

            ereignisse = []
            for i, ev in enumerate(erg.ereignisse):
                ids = [self._fid(f) for f in ev.fotos]
                key = ids[0]
                behalten = [x for x in ids if einst[x]["behalten"]]
                standardname = ev.titel
                ereignisse.append({
                    "index": i,
                    "key": key,
                    "nummer": ev.nummer,
                    "name": self.ereignis_namen.get(key, ""),
                    "titel": standardname,
                    "von": ev.start.isoformat(timespec="minutes"),
                    "bis": ev.ende.isoformat(timespec="minutes"),
                    "anzahl": len(ids),
                    "im_buch": len(behalten),
                    "vorschau": (behalten or ids)[:8:max(1, len(behalten or ids) // 8)][:8],
                    "ohne_zeit": sum(1 for f in ev.fotos if f.zeit_quelle == "datei"),
                    "seiten": seitenplan["seiten_je_ereignis"][i],
                })

            serien = []
            for nr, g in enumerate(erg.serien):
                ids = [self._fid(f) for f in g.fotos]
                serien.append({
                    "nr": nr,
                    "ids": ids,
                    "vorschlag": self._fid(g.vorschlag),
                    "entschieden": any(i in self.entscheidungen for i in ids),
                    "behalten": [i for i in ids if einst[i]["behalten"]],
                    "ereignis": self._ereignis_von_id[ids[0]],
                })

            fotos_kompakt = {}
            for fid, f in self.fotos.items():
                e = einst[fid]
                fotos_kompakt[fid] = {
                    "n": f.name,
                    "z": f.zeit.strftime("%d.%m.%Y %H:%M"),
                    "f": f.format,
                    "mp": round(f.megapixel, 1),
                    "b": e["behalten"],
                    "g": e["grund"],
                    "e": e["eigene"],
                    "ev": self._ereignis_von_id[fid],
                    "x": f.fehler,
                    "zq": f.zeit_quelle,
                    "raw": f.ist_raw,
                }

            entfernt_gruende: dict[str, int] = {}
            for v in einst.values():
                if not v["behalten"]:
                    g = v["grund"].split(",")[0]
                    entfernt_gruende[g] = entfernt_gruende.get(g, 0) + 1

            return {
                "programmversion": __version__,
                "ordner": str(self.ordner),
                "name": self.ordner.name,
                "schritt": self.schritt,
                "analysiert": self.analysiert_am is not None and bool(self.fotos),
                "analysiert_am": self.analysiert_am,
                "analyse_dauer": self.analyse_dauer,
                "einstellungen": self.einstellungen,
                "profil": {
                    "name": profil.name,
                    "verifiziert": profil.verifiziert,
                    "hinweis": profil.hinweis,
                    "seiten_min": profil.seiten_min,
                    "seiten_schritt": profil.seiten_schritt,
                    "seiten_max": profil.seiten_max,
                    "auswahl": PROFILE,
                },
                "zahlen": {
                    "dateien": len(fotos),
                    "lesbar": len(lesbar),
                    "nicht_lesbar": len(fotos) - len(lesbar),
                    "endungen": endungen,
                    "ohne_aufnahmezeit": sum(1 for f in lesbar if f.zeit_quelle == "datei"),
                    "ereignisse": len(erg.ereignisse),
                    "exakte_gruppen": len(erg.exakt),
                    "serien": len(erg.serien),
                    "serien_entschieden": sum(1 for s in serien if s["entschieden"]),
                    "screenshots": len(erg.screenshots),
                    "im_buch": im_buch,
                    "entfernt": len(fotos) - im_buch,
                    "entfernt_gruende": entfernt_gruende,
                    "eigene_entscheidungen": len(self.entscheidungen),
                },
                "ereignisse": ereignisse,
                "serien": serien,
                "fotos": fotos_kompakt,
                "seitenplan": seitenplan,
                "vorschlaege": self.vorschlaege(seitenplan),
            }

    # ------------------------------------------------------------------ Vorschaubilder

    def vorschau(self, fid: str, kante: int = 320) -> bytes:
        kante = max(64, min(1600, int(kante)))
        if fid not in self.fotos:
            raise ProjektFehler("Unbekanntes Foto")
        ordner = self.daten / "vorschau"
        ziel = ordner / f"{fid}_{kante}.jpg"
        if ziel.exists():
            return ziel.read_bytes()
        f = self.fotos[fid]
        try:
            img, _raw = _oeffne(Path(f.pfad))
            with img:
                if img.format == "JPEG":
                    img.draft("RGB", (kante * 2, kante * 2))
                img = ImageOps.exif_transpose(img)
                img.thumbnail((kante, kante))
                rgb = img.convert("RGB")
        except Exception:
            rgb = Image.new("RGB", (kante, int(kante * 0.75)), (200, 205, 210))
        puffer = io.BytesIO()
        rgb.save(puffer, "JPEG", quality=82)
        daten = puffer.getvalue()
        try:
            ordner.mkdir(parents=True, exist_ok=True)
            ziel.write_bytes(daten)
        except OSError:
            pass
        return daten

    # ------------------------------------------------------------------ Export

    def export_ziel_vorschlag(self) -> Path:
        return self.ordner / "fotobuch-export"

    def exportieren(self, ziel: str | Path | None = None, bilder_kopieren: bool = True) -> dict:
        """Kopiert die Fotos im Buch nummeriert in Ereignis-Ordner und schreibt die Seitenliste.

        Originale bleiben unverändert. Ein bestehender, nicht leerer Zielordner wird nie überschrieben:
        stattdessen entsteht ein neuer Unterordner mit Zeitstempel.
        """
        import shutil

        with self.lock:
            ziel = Path(ziel) if ziel else self.export_ziel_vorschlag()
            if ziel.exists() and any(ziel.iterdir()):
                ziel = ziel / f"export-{datetime.now():%Y%m%d-%H%M%S}"
            ziel.mkdir(parents=True, exist_ok=True)
            plan = self.seitenplan()
            zahlen = self._ereignis_zahlen()
            fps = int(self.einstellungen["fotos_pro_seite"])
            getrennt = bool(self.einstellungen["ereignisse_getrennt"])

            reihenfolge: list[tuple[int, str, str]] = []  # (Ereignis-Index, Foto-ID, Dateiname im Export)
            nr = 0
            kopiert = 0
            fehler: list[str] = []
            for ei, (_ids, buch) in enumerate(zahlen):
                if not buch:
                    continue
                key = self.erg_key(ei)
                name = self.ereignis_namen.get(key) or self.ergebnis.ereignisse[ei].titel
                ordnername = _sauber(f"{ei + 1:02d}_{name}")[:80]
                for fid in buch:
                    nr += 1
                    f = self.fotos[fid]
                    zielname = f"{nr:04d}_{_sauber(f.name)}"
                    rel = Path(ordnername) / zielname
                    if bilder_kopieren:
                        (ziel / ordnername).mkdir(parents=True, exist_ok=True)
                        try:
                            shutil.copy2(f.pfad, ziel / rel)
                            kopiert += 1
                        except OSError as e:
                            fehler.append(f"{f.name}: {e}")
                    reihenfolge.append((ei, fid, rel.as_posix()))

            # Seitenliste: wer steht auf welcher Seite
            seiten = seitenzuordnung([len(b) for _a, b in zahlen], fps, getrennt)
            zeilen = []
            pos = 0
            for seitennr, (_ei, groessen) in enumerate(seiten, 1):
                for _ in range(groessen[0]):
                    ei, fid, rel = reihenfolge[pos]
                    f = self.fotos[fid]
                    name = self.ereignis_namen.get(self.erg_key(ei)) or self.ergebnis.ereignisse[ei].titel
                    zeilen.append([seitennr, name, rel, f.name, f.zeit.strftime("%Y-%m-%d %H:%M")])
                    pos += 1
            for leer in range(len(seiten) + 1, plan["buch_seiten"] + 1):
                zeilen.append([leer, "(frei)", "", "", ""])
            with open(ziel / "seitenliste.csv", "w", newline="", encoding="utf-8-sig") as fh:
                w = csv.writer(fh, delimiter=";")
                w.writerow(["Seite", "Ereignis", "Datei im Export", "Originalname", "Aufnahmezeit"])
                w.writerows(zeilen)

            (ziel / "LIESMICH.txt").write_text(
                "Fotobuch-Assistent: Export\n"
                f"Erstellt am {datetime.now():%d.%m.%Y %H:%M}\n\n"
                f"Fotos im Buch: {nr}\n"
                f"Seiten (benötigt / Buch): {plan['seiten_benoetigt']} / {plan['buch_seiten']}\n"
                f"Fotos pro Seite: {fps}, Ereignisse beginnen auf neuer Seite: {'ja' if getrennt else 'nein'}\n\n"
                "So geht es weiter:\n"
                "1. Öffne die Software deines Druckdienstes und lade die Fotos aus den nummerierten Ordnern hoch.\n"
                "   Die Nummern am Dateianfang geben die Reihenfolge vor.\n"
                "2. In seitenliste.csv steht, welches Foto auf welche Seite gehört.\n"
                "3. Die Originalfotos wurden nicht verändert.\n",
                encoding="utf-8",
            )
            return {
                "ziel": str(ziel),
                "fotos": nr,
                "kopiert": kopiert,
                "seiten": plan["buch_seiten"],
                "fehler": fehler,
            }

    def erg_key(self, ereignis_index: int) -> str:
        return self._fid(self.ergebnis.ereignisse[ereignis_index].fotos[0])
