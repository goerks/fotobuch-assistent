"""Analyse-Lauf und Berichte. Es werden nie Bilder weitergegeben, nur Zahlen und Listen."""

from __future__ import annotations

import csv
import json
import os
import platform
import shutil
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

from . import __version__
from .duplicates import Gruppe, finde_aehnliche, finde_exakte
from .events import Ereignis, bilde_ereignisse
from .pages import Profil, seitenplan
from .scanner import HEIC_OK, RAW_OK, Foto, finde_fotos, scanne


@dataclass
class Optionen:
    luecke_stunden: float = 6.0
    max_km: float = 50.0
    aehnlich_abstand: int = 6
    serie_minuten: float = 15.0
    fotos_pro_seite: float = 3.0
    profil: Profil | None = None
    threads: int | None = None


@dataclass
class Ergebnis:
    ordner: str
    fotos: list[Foto]
    ereignisse: list[Ereignis]
    exakt: list[Gruppe]
    serien: list[Gruppe]
    zeiten: dict[str, float]
    optionen: Optionen
    # Abgeleitet
    zu_entfernen_exakt: set[str] = field(default_factory=set)
    zu_entfernen_serien: set[str] = field(default_factory=set)
    screenshots: set[str] = field(default_factory=set)


def ram_gb() -> float | None:
    try:
        if os.name == "nt":
            import ctypes

            class MEM(ctypes.Structure):
                _fields_ = [
                    ("dwLength", ctypes.c_ulong),
                    ("dwMemoryLoad", ctypes.c_ulong),
                    ("ullTotalPhys", ctypes.c_ulonglong),
                    ("ullAvailPhys", ctypes.c_ulonglong),
                    ("ullTotalPageFile", ctypes.c_ulonglong),
                    ("ullAvailPageFile", ctypes.c_ulonglong),
                    ("ullTotalVirtual", ctypes.c_ulonglong),
                    ("ullAvailVirtual", ctypes.c_ulonglong),
                    ("sullAvailExtendedVirtual", ctypes.c_ulonglong),
                ]

            m = MEM()
            m.dwLength = ctypes.sizeof(MEM)
            ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(m))
            return round(m.ullTotalPhys / 1024**3, 1)
        seiten = os.sysconf("SC_PHYS_PAGES")
        groesse = os.sysconf("SC_PAGE_SIZE")
        return round(seiten * groesse / 1024**3, 1)
    except Exception:
        return None


def hardware() -> dict:
    return {
        "system": f"{platform.system()} {platform.release()} ({platform.version()})",
        "prozessor": platform.processor() or platform.machine(),
        "kerne": os.cpu_count(),
        "ram_gb": ram_gb(),
        "heic_unterstuetzt": HEIC_OK,
        "raw_unterstuetzt": RAW_OK,
        "programmversion": __version__,
    }


def analysiere_ordner(
    ordner: str | Path,
    opt: Optionen,
    fortschritt: Callable[[int, int], None] | None = None,
) -> Ergebnis:
    zeiten: dict[str, float] = {}
    t0 = time.perf_counter()
    pfade = finde_fotos(ordner)
    zeiten["suchen"] = time.perf_counter() - t0

    t = time.perf_counter()
    fotos = scanne(pfade, threads=opt.threads, fortschritt=fortschritt)
    zeiten["einlesen_und_analysieren"] = time.perf_counter() - t

    t = time.perf_counter()
    ereignisse = bilde_ereignisse(fotos, opt.luecke_stunden, opt.max_km)
    zeiten["ereignisse"] = time.perf_counter() - t

    t = time.perf_counter()
    exakt = finde_exakte(fotos)
    zeiten["exakte_duplikate"] = time.perf_counter() - t

    t = time.perf_counter()
    serien = finde_aehnliche(fotos, opt.aehnlich_abstand, opt.serie_minuten * 60)
    zeiten["serien"] = time.perf_counter() - t
    zeiten["gesamt"] = time.perf_counter() - t0

    erg = Ergebnis(str(ordner), fotos, ereignisse, exakt, serien, zeiten, opt)
    for g in exakt:
        erg.zu_entfernen_exakt.update(f.pfad for f in g.fotos if f is not g.vorschlag)
    for g in serien:
        erg.zu_entfernen_serien.update(f.pfad for f in g.fotos if f is not g.vorschlag)
    erg.screenshots = {f.pfad for f in fotos if f.screenshot_verdacht}
    return erg


def _behalten(erg: Ergebnis, ohne_exakt=True, ohne_serien=True, ohne_screens=True) -> list[Foto]:
    weg: set[str] = set()
    if ohne_exakt:
        weg |= erg.zu_entfernen_exakt
    if ohne_serien:
        weg |= erg.zu_entfernen_serien
    if ohne_screens:
        weg |= erg.screenshots
    return [f for f in erg.fotos if f.pfad not in weg and not f.fehler]


def zusammenfassung(erg: Ergebnis) -> dict:
    fotos = erg.fotos
    ok = [f for f in fotos if not f.fehler]
    n = len(fotos)
    gesamt_s = erg.zeiten.get("gesamt", 0.0)
    analyse_s = erg.zeiten.get("einlesen_und_analysieren", 0.0)
    pro_1000 = (analyse_s / n * 1000) if n else 0.0
    formate = {"quer": 0, "hoch": 0, "quadrat": 0, "unbekannt": 0}
    for f in ok:
        formate[f.format] += 1
    endungen: dict[str, int] = {}
    for f in fotos:
        endungen[f.endung] = endungen.get(f.endung, 0) + 1
    ohne_zeit = sum(1 for f in ok if f.zeit_quelle == "datei")
    kameras: dict[str, int] = {}
    for f in ok:
        kameras[f.kamera or "(unbekannt)"] = kameras.get(f.kamera or "(unbekannt)", 0) + 1

    varianten = {
        "alle_lesbaren": len(ok),
        "ohne_exakte_duplikate": len(_behalten(erg, True, False, False)),
        "ohne_duplikate_und_screenshots": len(_behalten(erg, True, False, True)),
        "ohne_duplikate_serien_und_screenshots": len(_behalten(erg, True, True, True)),
    }
    profil = erg.optionen.profil
    seiten = {}
    if profil:
        for k, anz in varianten.items():
            seiten[k] = {
                f"{fps}_pro_seite": seitenplan(anz, fps, profil) for fps in (2, 3, 4, 5)
            }
    return {
        "ordner_name": Path(erg.ordner).name,
        "anzahl_dateien": n,
        "nicht_lesbar": n - len(ok),
        "endungen": endungen,
        "formate": formate,
        "ohne_aufnahmezeit": ohne_zeit,
        "kameras": kameras,
        "ereignisse": len(erg.ereignisse),
        "exakte_duplikat_gruppen": len(erg.exakt),
        "exakte_duplikat_ueberzaehlig": len(erg.zu_entfernen_exakt),
        "serien_gruppen": len(erg.serien),
        "serien_ueberzaehlig": len(erg.zu_entfernen_serien),
        "screenshot_verdacht": len(erg.screenshots),
        "varianten_fotozahl": varianten,
        "seitenplan": seiten,
        "profil": None
        if not profil
        else {"name": profil.name, "verifiziert": profil.verifiziert, "seiten_min": profil.seiten_min,
              "seiten_schritt": profil.seiten_schritt, "seiten_max": profil.seiten_max, "hinweis": profil.hinweis},
        "zeiten_sekunden": {k: round(v, 2) for k, v in erg.zeiten.items()},
        "analysezeit_pro_1000_fotos_sekunden": round(pro_1000, 1),
        "gesamtdauer_sekunden": round(gesamt_s, 1),
        "einstellungen": {
            "luecke_stunden": erg.optionen.luecke_stunden,
            "max_km": erg.optionen.max_km,
            "aehnlich_abstand": erg.optionen.aehnlich_abstand,
            "serie_minuten": erg.optionen.serie_minuten,
            "fotos_pro_seite": erg.optionen.fotos_pro_seite,
        },
    }


def _anonym_namen(erg: Ergebnis) -> dict[str, str]:
    return {f.pfad: f"foto-{i:04d}{f.endung}" for i, f in enumerate(sorted(erg.fotos, key=lambda f: f.pfad.lower()), 1)}


def schreibe_berichte(erg: Ergebnis, ziel: str | Path) -> dict[str, Path]:
    """Schreibt vollständige lokale Dateien und einen teilbaren Bericht ohne Namen, Pfade und GPS."""
    ziel = Path(ziel)
    ziel.mkdir(parents=True, exist_ok=True)
    zus = zusammenfassung(erg)
    hw = hardware()
    anon = _anonym_namen(erg)
    dateien: dict[str, Path] = {}

    # 1) Lokal: vollständige Liste mit Namen
    ereignis_von: dict[str, int] = {}
    for e in erg.ereignisse:
        for f in e.fotos:
            ereignis_von[f.pfad] = e.nummer
    exakt_id = {f.pfad: i for i, g in enumerate(erg.exakt, 1) for f in g.fotos}
    serie_id = {f.pfad: i for i, g in enumerate(erg.serien, 1) for f in g.fotos}
    vorschlag = {g.vorschlag.pfad for g in erg.exakt + erg.serien if g.vorschlag}
    wurzel = Path(erg.ordner)
    pfad_csv = ziel / "reihenfolge.csv"
    with open(pfad_csv, "w", newline="", encoding="utf-8-sig") as fh:
        w = csv.writer(fh, delimiter=";")
        w.writerow(["ereignis", "datei", "ordner", "zeit", "zeit_quelle", "format", "megapixel", "kamera",
                    "exakt_gruppe", "serie_gruppe", "vorschlag_behalten", "screenshot_verdacht", "fehler"])
        for e in erg.ereignisse:
            for f in e.fotos:
                try:
                    rel = str(Path(f.pfad).parent.relative_to(wurzel))
                except ValueError:
                    rel = str(Path(f.pfad).parent)
                w.writerow([e.nummer, f.name, rel, f"{f.zeit:%Y-%m-%d %H:%M:%S}", f.zeit_quelle, f.format,
                            f"{f.megapixel:.1f}", f.kamera, exakt_id.get(f.pfad, ""), serie_id.get(f.pfad, ""),
                            "ja" if f.pfad in vorschlag else "", "ja" if f.screenshot_verdacht else "",
                            f.fehler])
    dateien["reihenfolge"] = pfad_csv

    # 2) Teilbar: nur Zahlen und anonyme Listen
    teilbar = {
        "hinweis": "Enthält keine Bilder, keine Dateinamen, keine Pfade und keine GPS-Daten.",
        "hardware": hw,
        "zusammenfassung": {k: v for k, v in zus.items() if k != "ordner_name"},
        "ereignisse": [
            {"nummer": e.nummer, "fotos": len(e.fotos), "von": f"{e.start:%Y-%m-%d %H:%M}",
             "bis": f"{e.ende:%Y-%m-%d %H:%M}"} for e in erg.ereignisse
        ],
        "exakte_gruppen": [[anon[f.pfad] for f in g.fotos] for g in erg.exakt],
        "serien_gruppen": [
            {"fotos": [anon[f.pfad] for f in g.fotos], "vorschlag": anon[g.vorschlag.pfad]} for g in erg.serien
        ],
        "fehler": [{"foto": anon[f.pfad], "fehler": f.fehler} for f in erg.fotos if f.fehler],
        "messwerte": [
            {"foto": anon[f.pfad], "format": f.format, "megapixel": round(f.megapixel, 1),
             "schaerfe": round(f.schaerfe, 1), "helligkeit": round(f.helligkeit, 1)}
            for f in erg.fotos if not f.fehler
        ],
    }
    pj = ziel / "bericht-teilbar.json"
    pj.write_text(json.dumps(teilbar, ensure_ascii=False, indent=2), encoding="utf-8")
    dateien["teilbar_json"] = pj

    zuord = ziel / "zuordnung-NICHT-TEILEN.csv"
    with open(zuord, "w", newline="", encoding="utf-8-sig") as fh:
        w = csv.writer(fh, delimiter=";")
        w.writerow(["anonymer_name", "originalpfad"])
        for pfad, name in sorted(anon.items(), key=lambda kv: kv[1]):
            w.writerow([name, pfad])
    dateien["zuordnung"] = zuord

    pm = ziel / "bericht.md"
    pm.write_text(_markdown(erg, zus, hw), encoding="utf-8")
    dateien["bericht"] = pm
    return dateien


def _markdown(erg: Ergebnis, zus: dict, hw: dict) -> str:
    z = zus
    v = z["varianten_fotozahl"]
    zeilen = [
        f"# Fotobuch-Assistent: Bericht ({z['ordner_name']})",
        "",
        "Dieser Bericht enthält keine Bilder. Zum Teilen bitte `bericht-teilbar.json` verwenden.",
        "",
        "## Rechner",
        f"- System: {hw['system']}",
        f"- Prozessor: {hw['prozessor']}, Kerne: {hw['kerne']}, RAM: {hw['ram_gb']} GB",
        f"- HEIC: {'ja' if hw['heic_unterstuetzt'] else 'nein'}, RAW-Vorschau: {'ja' if hw['raw_unterstuetzt'] else 'nein'}",
        "",
        "## Tempo",
        f"- Dateien: {z['anzahl_dateien']}, Gesamtdauer: {z['gesamtdauer_sekunden']} s",
        f"- Analyse pro 1000 Fotos: {z['analysezeit_pro_1000_fotos_sekunden']} s "
        f"({z['analysezeit_pro_1000_fotos_sekunden'] / 60:.1f} min), Ziel: unter 15 min",
        "",
        "## Inhalt",
        f"- Endungen: {z['endungen']}",
        f"- Formate: {z['formate']}",
        f"- Nicht lesbar: {z['nicht_lesbar']}, ohne Aufnahmezeit: {z['ohne_aufnahmezeit']}",
        f"- Ereignisse: {z['ereignisse']}",
        f"- Exakte Duplikate: {z['exakte_duplikat_gruppen']} Gruppen, {z['exakte_duplikat_ueberzaehlig']} überzählig",
        f"- Serien (ähnlich): {z['serien_gruppen']} Gruppen, {z['serien_ueberzaehlig']} überzählig (Vorschlag)",
        f"- Screenshot-Verdacht: {z['screenshot_verdacht']}",
        "",
        "## Fotozahl nach Reduktion (Vorschläge)",
        f"- Alle lesbaren: {v['alle_lesbaren']}",
        f"- Ohne exakte Duplikate: {v['ohne_exakte_duplikate']}",
        f"- Ohne Duplikate und Screenshots: {v['ohne_duplikate_und_screenshots']}",
        f"- Ohne Duplikate, Serien-Reste und Screenshots: {v['ohne_duplikate_serien_und_screenshots']}",
        "",
    ]
    if z["profil"]:
        p = z["profil"]
        zeilen += [
            f"## Seitenplan (Profil: {p['name']}, {'verifiziert' if p['verifiziert'] else 'UNVERIFIZIERT: Annahmen'})",
            f"Seiten min {p['seiten_min']}, Schritt {p['seiten_schritt']}, max {p['seiten_max']}",
            "",
            "| Fotozahl | Fotos/Seite | Seiten nötig | Buchseiten | Es fehlen | Weglassen für kleinere Stufe |",
            "|---|---|---|---|---|---|",
        ]
        wahl = z["seitenplan"].get("ohne_duplikate_serien_und_screenshots", {})
        for key, s in wahl.items():
            zeilen.append(
                f"| {s['fotos']} | {s['fotos_pro_seite']:g} | {s['seiten_benoetigt']} | {s['buch_seiten']} | "
                f"{s['fehlende_fotos']} | {s['ueberzaehlige_fotos_fuer_kleinere_stufe']} |"
            )
        zeilen.append("")
    zeilen += ["## Ereignisse", ""]
    for e in erg.ereignisse:
        zeilen.append(f"- Ereignis {e.nummer:02d}: {e.titel}, {len(e.fotos)} Fotos")
    zeilen.append("")
    return "\n".join(zeilen)


def exportiere_sortiert(
    erg: Ergebnis,
    ziel: str | Path,
    ohne_exakt: bool = True,
    ohne_serien: bool = False,
    ohne_screenshots: bool = True,
) -> int:
    """Kopiert Fotos nummeriert in Ereignis-Ordner. Originale bleiben unverändert."""
    ziel = Path(ziel)
    weg: set[str] = set()
    if ohne_exakt:
        weg |= erg.zu_entfernen_exakt
    if ohne_serien:
        weg |= erg.zu_entfernen_serien
    if ohne_screenshots:
        weg |= erg.screenshots
    zaehler = 0
    gesamt = 0
    for e in erg.ereignisse:
        sichtbar = [f for f in e.fotos if f.pfad not in weg and not f.fehler]
        if not sichtbar:
            continue
        ordner = ziel / f"{e.nummer:02d}_{e.start:%Y-%m-%d}"
        ordner.mkdir(parents=True, exist_ok=True)
        for f in sichtbar:
            gesamt += 1
            shutil.copy2(f.pfad, ordner / f"{gesamt:04d}_{f.name}")
            zaehler += 1
    return zaehler
