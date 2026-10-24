"""Lokaler Webserver der App. Hört nur auf 127.0.0.1, zufälliger Port, Zugriff nur mit Token.

Nichts verlässt den Rechner. Die Oberfläche ist reines HTML/CSS/JS ohne externe Ressourcen.
"""

from __future__ import annotations

import json
import mimetypes
import secrets
import threading
import traceback
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib import resources
from pathlib import Path
from urllib.parse import parse_qs, urlparse

from . import __version__
from .projekt import Projekt, ProjektFehler

MAX_BODY = 1_000_000


def web_ordner() -> Path:
    return Path(str(resources.files("fotobuch").joinpath("web")))


class App:
    """Zustand der laufenden App: aktuelles Projekt, Analysefortschritt, Token."""

    def __init__(self, ordner_dialog=None):
        self.token = secrets.token_urlsafe(24)
        self.projekt: Projekt | None = None
        self.lock = threading.RLock()
        self.job = {"laeuft": False, "phase": "", "fertig": 0, "gesamt": 0, "fehler": None}
        self.ordner_dialog = ordner_dialog
        self.beenden = threading.Event()

    def oeffne(self, ordner: str) -> Projekt:
        with self.lock:
            if self.job["laeuft"]:
                raise ProjektFehler("Es läuft gerade eine Analyse.")
            self.projekt = Projekt(ordner)
            self.job = {"laeuft": False, "phase": "", "fertig": 0, "gesamt": 0, "fehler": None}
            return self.projekt

    def starte_analyse(self) -> None:
        with self.lock:
            if not self.projekt:
                raise ProjektFehler("Zuerst einen Ordner wählen.")
            if self.job["laeuft"]:
                return
            self.job = {"laeuft": True, "phase": "suchen", "fertig": 0, "gesamt": 0, "fehler": None}
            projekt = self.projekt

        def fortschritt(phase, fertig, gesamt):
            self.job.update(phase=phase, fertig=fertig, gesamt=gesamt)

        def lauf():
            try:
                projekt.analysieren(fortschritt)
                if projekt.schritt < 2:
                    projekt.setze_schritt(2)
                self.job.update(laeuft=False, phase="fertig")
            except Exception as e:  # nie den Server mit abstürzen lassen
                traceback.print_exc()
                self.job.update(laeuft=False, phase="fehler", fehler=f"{type(e).__name__}: {e}")

        threading.Thread(target=lauf, daemon=True).start()


def baue_handler(app: App):
    class Handler(BaseHTTPRequestHandler):
        server_version = f"Fotobuch/{__version__}"

        def log_message(self, *args):  # still
            pass

        # -------- Hilfen
        def _host_ok(self) -> bool:
            host = (self.headers.get("Host") or "").split(":")[0]
            return host in ("127.0.0.1", "localhost")

        def _token_ok(self, query: dict) -> bool:
            t = self.headers.get("X-Fotobuch-Token") or (query.get("t") or [""])[0]
            return secrets.compare_digest(t, app.token)

        def _json(self, daten, status=200):
            body = json.dumps(daten, ensure_ascii=False).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def _fehler(self, status, text):
            self._json({"fehler": text}, status)

        def _body(self) -> dict:
            n = int(self.headers.get("Content-Length") or 0)
            if n > MAX_BODY:
                raise ProjektFehler("Anfrage zu gross")
            if n == 0:
                return {}
            try:
                d = json.loads(self.rfile.read(n).decode("utf-8"))
            except Exception:
                raise ProjektFehler("Ungültige Anfrage")
            if not isinstance(d, dict):
                raise ProjektFehler("Ungültige Anfrage")
            return d

        def _projekt(self) -> Projekt:
            if not app.projekt:
                raise ProjektFehler("Noch kein Ordner gewählt.")
            return app.projekt

        # -------- GET
        def do_GET(self):
            if not self._host_ok():
                return self._fehler(403, "Host nicht erlaubt")
            u = urlparse(self.path)
            q = parse_qs(u.query)
            pfad = u.path
            if not pfad.startswith("/api/"):
                return self._statisch(pfad)
            if not self._token_ok(q):
                return self._fehler(403, "Kein Zugriff")
            try:
                if pfad == "/api/status":
                    return self._json(self._status())
                if pfad == "/api/ansicht":
                    return self._json(self._projekt().ansicht())
                if pfad.startswith("/api/vorschau/"):
                    fid = pfad.rsplit("/", 1)[-1]
                    kante = int((q.get("k") or ["320"])[0])
                    daten = self._projekt().vorschau(fid, kante)
                    self.send_response(200)
                    self.send_header("Content-Type", "image/jpeg")
                    self.send_header("Content-Length", str(len(daten)))
                    self.send_header("Cache-Control", "private, max-age=3600")
                    self.end_headers()
                    self.wfile.write(daten)
                    return
                return self._fehler(404, "Unbekannt")
            except ProjektFehler as e:
                return self._fehler(400, str(e))
            except ValueError:
                return self._fehler(400, "Ungültiger Wert")
            except Exception as e:
                traceback.print_exc()
                return self._fehler(500, f"Interner Fehler: {type(e).__name__}")

        def _status(self) -> dict:
            p = app.projekt
            return {
                "version": __version__,
                "ordner": str(p.ordner) if p else None,
                "job": app.job,
                "hat_projekt": p is not None,
                "kann_dialog": app.ordner_dialog is not None,
            }

        def _statisch(self, pfad: str):
            if pfad in ("", "/"):
                pfad = "/index.html"
            rel = pfad.lstrip("/")
            wurzel = web_ordner().resolve()
            ziel = (wurzel / rel).resolve()
            if wurzel not in ziel.parents or not ziel.is_file():
                return self._fehler(404, "Nicht gefunden")
            daten = ziel.read_bytes()
            if ziel.name == "index.html":
                daten = daten.replace(b"__TOKEN__", app.token.encode("ascii"))
            ctype = mimetypes.guess_type(ziel.name)[0] or "application/octet-stream"
            if ctype.startswith("text/") or ctype in ("application/javascript",):
                ctype += "; charset=utf-8"
            self.send_response(200)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(daten)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header(
                "Content-Security-Policy",
                "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'",
            )
            self.end_headers()
            self.wfile.write(daten)

        # -------- POST
        def do_POST(self):
            if not self._host_ok():
                return self._fehler(403, "Host nicht erlaubt")
            u = urlparse(self.path)
            if not u.path.startswith("/api/") or not self._token_ok(parse_qs(u.query)):
                return self._fehler(403, "Kein Zugriff")
            if self.headers.get("Content-Type", "").split(";")[0] != "application/json":
                return self._fehler(415, "Nur JSON erlaubt")
            try:
                d = self._body()
                return self._json(self._aktion(u.path, d))
            except ProjektFehler as e:
                return self._fehler(400, str(e))
            except (KeyError, ValueError, TypeError):
                return self._fehler(400, "Ungültige Anfrage")
            except Exception as e:
                traceback.print_exc()
                return self._fehler(500, f"Interner Fehler: {type(e).__name__}")

        def _aktion(self, pfad: str, d: dict) -> dict:
            if pfad == "/api/ordner/waehlen":
                if not app.ordner_dialog:
                    raise ProjektFehler("Auf diesem Rechner ist kein Ordner-Dialog verfügbar. Gib den Pfad ein.")
                gewaehlt = app.ordner_dialog()
                if not gewaehlt:
                    return {"ok": True, "abgebrochen": True}
                app.oeffne(gewaehlt)
                return {"ok": True, "ordner": gewaehlt}
            if pfad == "/api/ordner/oeffnen":
                ordner = str(d.get("pfad", "")).strip().strip('"')
                if not ordner:
                    raise ProjektFehler("Bitte einen Ordner angeben.")
                app.oeffne(ordner)
                return {"ok": True, "ordner": ordner}
            if pfad == "/api/analyse/start":
                app.starte_analyse()
                return {"ok": True}
            if pfad == "/api/beenden":
                app.beenden.set()
                return {"ok": True}
            p = self._projekt()
            if pfad == "/api/einstellungen":
                p.setze_einstellungen(d)
                return {"ok": True}
            if pfad == "/api/schritt":
                p.setze_schritt(int(d["schritt"]))
                return {"ok": True}
            if pfad == "/api/foto":
                p.setze_foto(str(d["id"]), str(d["aktion"]))
                return {"ok": True}
            if pfad == "/api/serie":
                p.serie_entscheiden(int(d["nr"]), d["waehle"])
                return {"ok": True}
            if pfad == "/api/serien/alle-vorschlaege":
                p.alle_serien_auf_vorschlag()
                return {"ok": True}
            if pfad == "/api/entscheidungen/zuruecksetzen":
                p.alle_entscheidungen_zuruecksetzen()
                return {"ok": True}
            if pfad == "/api/ereignis/name":
                p.ereignis_umbenennen(str(d["key"]), str(d.get("name", "")))
                return {"ok": True}
            if pfad == "/api/vorschlag/anwenden":
                n = p.vorschlag_anwenden(str(d["art"]), d.get("ids"))
                return {"ok": True, "anzahl": n}
            if pfad == "/api/export":
                return {"ok": True, **p.exportieren(d.get("ziel") or None)}
            raise ProjektFehler("Unbekannte Aktion")

    return Handler


def starte_server(app: App, port: int = 0) -> ThreadingHTTPServer:
    httpd = ThreadingHTTPServer(("127.0.0.1", port), baue_handler(app))
    httpd.daemon_threads = True
    return httpd


def ordner_dialog_tk() -> str | None:
    """Windows-Ordnerdialog über tkinter. Gibt None zurück, wenn abgebrochen."""
    import tkinter
    from tkinter import filedialog

    root = tkinter.Tk()
    root.withdraw()
    try:
        root.attributes("-topmost", True)
    except Exception:
        pass
    try:
        return filedialog.askdirectory(title="Ordner mit den Fotos wählen") or None
    finally:
        root.destroy()


def url(httpd: ThreadingHTTPServer) -> str:
    return f"http://127.0.0.1:{httpd.server_address[1]}/"
