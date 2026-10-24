"""Kommandozeile und Doppelklick-Modus für die .exe."""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

from . import __version__
from .pages import lade_profil
from .report import Optionen, analysiere_ordner, exportiere_sortiert, schreibe_berichte, zusammenfassung


def _fortschritt_balken():
    letzte = [0.0]

    def f(i: int, n: int) -> None:
        jetzt = time.perf_counter()
        if i == n or jetzt - letzte[0] > 0.5:
            letzte[0] = jetzt
            breite = 30
            gefuellt = int(breite * i / max(n, 1))
            print(f"\r  [{'#' * gefuellt}{'.' * (breite - gefuellt)}] {i}/{n} Fotos", end="", flush=True)
            if i == n:
                print()

    return f


def _waehle_ordner_dialog() -> str | None:
    try:
        import tkinter as tk
        from tkinter import filedialog

        root = tk.Tk()
        root.withdraw()
        root.attributes("-topmost", True)
        pfad = filedialog.askdirectory(title="Ordner mit Fotos wählen")
        root.destroy()
        return pfad or None
    except Exception:
        return None


def _zeige_zusammenfassung(erg, zus, dateien) -> None:
    v = zus["varianten_fotozahl"]
    print()
    print(f"Fertig in {zus['gesamtdauer_sekunden']} s ({zus['analysezeit_pro_1000_fotos_sekunden']} s pro 1000 Fotos).")
    print(f"  Dateien: {zus['anzahl_dateien']} (nicht lesbar: {zus['nicht_lesbar']})")
    print(f"  Ereignisse: {zus['ereignisse']}")
    print(f"  Exakte Duplikate: {zus['exakte_duplikat_ueberzaehlig']} überzählig in {zus['exakte_duplikat_gruppen']} Gruppen")
    print(f"  Serien: {zus['serien_ueberzaehlig']} überzählig in {zus['serien_gruppen']} Gruppen")
    print(f"  Screenshot-Verdacht: {zus['screenshot_verdacht']}")
    print(f"  Fotos nach Reduktion (Vorschlag): {v['ohne_duplikate_serien_und_screenshots']}")
    print()
    print("Berichte:")
    for name, pfad in dateien.items():
        print(f"  {pfad}")
    print()
    print("Zum Teilen mit dem Entwickler nur 'bericht-teilbar.json' senden. Es enthält keine Bilder, Namen oder Pfade.")


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(
        prog="fotobuch",
        description="Fotobuch-Assistent: ordnet Fotos, findet Duplikate und rechnet Seiten. Alles lokal und offline.",
    )
    p.add_argument("--version", action="version", version=f"fotobuch {__version__}")
    sub = p.add_subparsers(dest="befehl")

    a = sub.add_parser("analyse", help="Ordner analysieren und Berichte schreiben")
    a.add_argument("ordner")
    a.add_argument("--ausgabe", help="Zielordner für Berichte (Standard: <ordner>/fotobuch-bericht)")
    a.add_argument("--luecke-stunden", type=float, default=6.0, help="Zeitlücke, ab der ein neues Ereignis beginnt")
    a.add_argument("--max-km", type=float, default=50.0, help="Ortssprung, ab dem ein neues Ereignis beginnt")
    a.add_argument("--aehnlich-abstand", type=int, default=6, help="Max. Hash-Abstand (0-64) für Serien")
    a.add_argument("--serie-minuten", type=float, default=15.0, help="Max. Zeitabstand innerhalb einer Serie")
    a.add_argument("--fotos-pro-seite", type=float, default=3.0)
    a.add_argument("--profil", default="allgemein", help="Anbieter-Profil (Name oder Pfad zu .json)")
    a.add_argument("--seiten-min", type=int)
    a.add_argument("--seiten-schritt", type=int)
    a.add_argument("--seiten-max", type=int)
    a.add_argument("--threads", type=int)

    e = sub.add_parser("export", help="Fotos nummeriert in Ereignis-Ordner kopieren (Originale bleiben)")
    e.add_argument("ordner")
    e.add_argument("ziel")
    e.add_argument("--ohne-serien", action="store_true", help="Nur den Vorschlag je Serie behalten")
    e.add_argument("--mit-screenshots", action="store_true")
    e.add_argument("--mit-exakten-duplikaten", action="store_true")
    e.add_argument("--luecke-stunden", type=float, default=6.0)

    ap = sub.add_parser("app", help="Oberfläche im Browser starten (Standard bei Doppelklick)")
    ap.add_argument("ordner", nargs="?", help="Fotoordner direkt öffnen (sonst Auswahl in der App)")
    ap.add_argument("--port", type=int, default=0)
    ap.add_argument("--kein-browser", action="store_true", help="Browser nicht automatisch öffnen")

    sub.add_parser("bericht", help="Alter Doppelklick-Modus: nur Berichte schreiben")

    args = p.parse_args(argv)

    if args.befehl is None:
        return _app(None, 0, False)
    if args.befehl == "app":
        return _app(args.ordner, args.port, not args.kein_browser)
    if args.befehl == "bericht":
        return _doppelklick()

    if args.befehl == "analyse":
        return _analyse(args)
    if args.befehl == "export":
        return _export(args)
    return 1


def _optionen(args) -> Optionen:
    profil = lade_profil(args.profil).mit_vorgaben(args.seiten_min, args.seiten_schritt, args.seiten_max)
    return Optionen(
        luecke_stunden=args.luecke_stunden,
        max_km=args.max_km,
        aehnlich_abstand=args.aehnlich_abstand,
        serie_minuten=args.serie_minuten,
        fotos_pro_seite=args.fotos_pro_seite,
        profil=profil,
        threads=args.threads,
    )


def _analyse(args) -> int:
    ordner = Path(args.ordner)
    if not ordner.is_dir():
        print(f"Ordner nicht gefunden: {ordner}", file=sys.stderr)
        return 2
    print(f"Analysiere {ordner} ...")
    erg = analysiere_ordner(ordner, _optionen(args), _fortschritt_balken())
    ziel = Path(args.ausgabe) if args.ausgabe else ordner / "fotobuch-bericht"
    dateien = schreibe_berichte(erg, ziel)
    _zeige_zusammenfassung(erg, zusammenfassung(erg), dateien)
    return 0


def _export(args) -> int:
    ordner = Path(args.ordner)
    if not ordner.is_dir():
        print(f"Ordner nicht gefunden: {ordner}", file=sys.stderr)
        return 2
    opt = Optionen(luecke_stunden=args.luecke_stunden, profil=lade_profil("allgemein").mit_vorgaben())
    print(f"Analysiere {ordner} ...")
    erg = analysiere_ordner(ordner, opt, _fortschritt_balken())
    n = exportiere_sortiert(
        erg,
        args.ziel,
        ohne_exakt=not args.mit_exakten_duplikaten,
        ohne_serien=args.ohne_serien,
        ohne_screenshots=not args.mit_screenshots,
    )
    print(f"{n} Fotos nach {args.ziel} kopiert. Die Originale wurden nicht verändert.")
    return 0


def _doppelklick() -> int:
    """Kein Argument: Ordner per Dialog wählen, analysieren, Berichte neben den Fotos ablegen."""
    print(f"Fotobuch-Assistent {__version__}")
    print("Alles läuft lokal auf diesem Rechner. Es werden keine Fotos hochgeladen.")
    print()
    ordner = _waehle_ordner_dialog()
    if not ordner:
        ordner = input("Ordner mit Fotos (Pfad eingeben): ").strip().strip('"')
    if not ordner or not Path(ordner).is_dir():
        print("Kein gültiger Ordner gewählt.")
        input("Enter drücken zum Beenden ...")
        return 2
    opt = Optionen(profil=lade_profil("allgemein").mit_vorgaben())
    print(f"Analysiere {ordner} ...")
    try:
        erg = analysiere_ordner(ordner, opt, _fortschritt_balken())
        dateien = schreibe_berichte(erg, Path(ordner) / "fotobuch-bericht")
        _zeige_zusammenfassung(erg, zusammenfassung(erg), dateien)
    except Exception as ex:  # Doppelklick-Fenster soll offen bleiben
        print(f"Fehler: {ex}")
        input("Enter drücken zum Beenden ...")
        return 1
    input("Enter drücken zum Beenden ...")
    return 0


def _app(ordner: str | None, port: int, browser: bool) -> int:
    """Startet den lokalen Server und öffnet die Oberfläche im Standardbrowser."""
    import threading
    import webbrowser

    from .server import App, ordner_dialog_tk, starte_server, url

    print(f"Fotobuch-Assistent {__version__}")
    print("Alles läuft lokal auf diesem Rechner. Es werden keine Fotos hochgeladen.")
    try:
        import tkinter  # noqa: F401
        dialog = ordner_dialog_tk
    except Exception:
        dialog = None
    app = App(ordner_dialog=dialog)
    if ordner:
        try:
            app.oeffne(ordner)
        except Exception as e:
            print(f"Ordner kann nicht geöffnet werden: {e}", file=sys.stderr)
            return 2
    httpd = starte_server(app, port)
    adresse = url(httpd)
    print()
    print(f"Die App läuft unter {adresse}")
    print("Dieses Fenster offen lassen, solange du arbeitest. Zum Beenden: Fenster schliessen oder Strg+C.")
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    if browser:
        try:
            webbrowser.open(adresse)
        except Exception:
            print("Der Browser konnte nicht automatisch geöffnet werden. Öffne die Adresse oben von Hand.")
    try:
        while not app.beenden.wait(0.5):
            pass
    except KeyboardInterrupt:
        pass
    httpd.shutdown()
    print("Beendet.")
    return 0
