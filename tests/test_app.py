"""Tests der App-Schicht: Projekt, lokaler Server, Sicherheit. Nur synthetische Bilder."""

import json
import threading
import urllib.error
import urllib.request
from datetime import datetime, timedelta
from pathlib import Path

import pytest

from conftest import motiv, speichere
from fotobuch.projekt import Projekt, ProjektFehler
from fotobuch.server import App, starte_server

T0 = datetime(2026, 7, 3, 9, 0)


def baue_ordner(d: Path) -> Path:
    for i in range(12):  # Tag 1: 12 verschiedene Fotos
        speichere(motiv(i), d / f"a{i:02d}.jpg", T0 + timedelta(minutes=i * 40))
    for i in range(6):  # Tag 2
        speichere(motiv(100 + i), d / f"b{i}.jpg", T0 + timedelta(days=1, minutes=i * 40))
    speichere(motiv(3), d / "a03_kopie.jpg", T0 + timedelta(minutes=3 * 40))  # exaktes Duplikat (gleiche Datei)
    (d / "a03_kopie.jpg").write_bytes((d / "a03.jpg").read_bytes())
    # Serie: zwei fast gleiche Fotos, eins unscharf
    from PIL import ImageFilter
    basis = motiv(500)
    speichere(basis, d / "s1.jpg", T0 + timedelta(days=1, hours=5))
    speichere(basis.filter(ImageFilter.GaussianBlur(3)), d / "s2.jpg", T0 + timedelta(days=1, hours=5, minutes=1))
    return d


@pytest.fixture
def projekt(tmp_path):
    p = Projekt(baue_ordner(tmp_path))
    p.analysieren()
    return p


def test_ansicht_zaehlt_richtig(projekt):
    a = projekt.ansicht()
    z = a["zahlen"]
    assert z["dateien"] == 21 and z["ereignisse"] == 2
    assert z["exakte_gruppen"] == 1 and z["serien"] == 1
    assert z["im_buch"] == 19  # 21 minus Duplikat minus unscharfes Serien-Bild
    s = a["serien"][0]
    assert a["fotos"][s["vorschlag"]]["n"] == "s1.jpg"  # das schärfere
    assert a["seitenplan"]["buch_seiten"] == 24


def test_entscheidungen_sind_umkehrbar_und_bleiben_gespeichert(projekt, tmp_path):
    a = projekt.ansicht()
    s = a["serien"][0]
    anderes = [i for i in s["ids"] if i != s["vorschlag"]][0]
    projekt.serie_entscheiden(0, anderes)
    b = projekt.ansicht()
    assert b["fotos"][anderes]["b"] and not b["fotos"][s["vorschlag"]]["b"]
    # Neustart: Entscheidung bleibt, ohne neu zu analysieren
    p2 = Projekt(tmp_path)
    c = p2.ansicht()
    assert c["fotos"][anderes]["b"]
    p2.serie_entscheiden(0, "vorschlag")
    assert p2.ansicht()["fotos"][s["vorschlag"]]["b"]
    # Beide behalten, keines behalten
    p2.serie_entscheiden(0, list(s["ids"]))
    assert p2.ansicht()["zahlen"]["im_buch"] == 20
    p2.serie_entscheiden(0, "keines")
    assert p2.ansicht()["zahlen"]["im_buch"] == 18
    with pytest.raises(ProjektFehler):
        p2.serie_entscheiden(0, "gibt-es-nicht")


def test_cache_liest_unveraenderte_fotos_nicht_neu(projekt, tmp_path, monkeypatch):
    gelesen = []
    import fotobuch.projekt as pm
    orig = pm.analysiere_foto
    monkeypatch.setattr(pm, "analysiere_foto", lambda p: (gelesen.append(p), orig(p))[1])
    projekt.analysieren()
    assert gelesen == []
    speichere(motiv(900), tmp_path / "neu.jpg", T0 + timedelta(days=3))
    projekt.analysieren()
    assert [g.name for g in gelesen] == ["neu.jpg"]
    assert projekt.ansicht()["zahlen"]["dateien"] == 22


def test_geloeschtes_foto_verschwindet_samt_entscheidung(projekt, tmp_path):
    fid = next(i for i, f in projekt.fotos.items() if f.name == "b0.jpg")
    projekt.setze_foto(fid, "entfernen")
    (tmp_path / "b0.jpg").unlink()
    projekt.analysieren()
    assert fid not in projekt.entscheidungen and fid not in projekt.fotos


def test_defekter_zustand_wird_beiseite_gelegt(tmp_path):
    baue_ordner(tmp_path)
    (tmp_path / ".fotobuch").mkdir()
    (tmp_path / ".fotobuch" / "projekt.json").write_text("{kaputt", encoding="utf-8")
    p = Projekt(tmp_path)  # darf nicht abstürzen
    assert not p.fotos
    assert (tmp_path / ".fotobuch" / "projekt.defekt").exists()


def test_einstellungen_werden_geprueft(projekt):
    with pytest.raises(ProjektFehler):
        projekt.setze_einstellungen({"fotos_pro_seite": 99})
    with pytest.raises(ProjektFehler):
        projekt.setze_einstellungen({"profil": "../../etc/passwd"})
    with pytest.raises(ProjektFehler):
        projekt.setze_einstellungen({"unbekannt": 1})
    with pytest.raises(ProjektFehler):
        projekt.setze_einstellungen({"luecke_stunden": "abc"})
    projekt.setze_einstellungen({"fotos_pro_seite": 6})
    assert projekt.ansicht()["seitenplan"]["fotos_pro_seite"] == 6


def test_luecke_aendert_ereignisse(projekt):
    projekt.setze_einstellungen({"luecke_stunden": 0.5})
    assert projekt.ansicht()["zahlen"]["ereignisse"] > 2
    projekt.setze_einstellungen({"luecke_stunden": 72})
    assert projekt.ansicht()["zahlen"]["ereignisse"] == 1


def test_vorschlag_kuerzen_trifft_die_kleinere_stufe_genau(tmp_path):
    # 30 Fotos, 3 pro Seite -> 10 Seiten. Stufe darunter (8 Seiten) = 24 Fotos, also 6 weglassen.
    for i in range(30):
        speichere(motiv(i), tmp_path / f"f{i:02d}.jpg", T0 + timedelta(minutes=i))
    p = Projekt(tmp_path)
    p.analysieren()
    p.setze_einstellungen({"seiten_min": 4, "seiten_schritt": 2, "seiten_max": 100})
    plan = p.ansicht()["seitenplan"]
    assert plan["seiten_benoetigt"] == 10 and plan["buch_seiten"] == 10
    assert plan["fehlende_fotos"] == 0 and plan["weglassen_fuer_kleinere_stufe"] == 6
    assert len(p.ansicht()["vorschlaege"]["kuerzen"]) == 6
    # Eigene "behalten"-Entscheidungen sind geschuetzt
    geschuetzt = p.ansicht()["vorschlaege"]["kuerzen"][0]
    p.setze_foto(geschuetzt, "behalten")
    assert geschuetzt not in p.ansicht()["vorschlaege"]["kuerzen"]
    assert p.vorschlag_anwenden("kuerzen") == 6
    plan = p.ansicht()["seitenplan"]
    assert plan["fotos"] == 24 and plan["seiten_benoetigt"] == 8 and plan["buch_seiten"] == 8
    assert plan["fehlende_fotos"] == 0


def test_vorschlag_auffuellen_holt_serienreste_zurueck(tmp_path):
    # 8 Einzelfotos + 2 Serien aus je 2 fast gleichen. 3 pro Seite, min 4, Schritt 2.
    from PIL import ImageFilter
    for i in range(8):
        speichere(motiv(i), tmp_path / f"e{i}.jpg", T0 + timedelta(hours=i * 7))
    for k in range(2):
        b = motiv(700 + k)
        speichere(b, tmp_path / f"s{k}a.jpg", T0 + timedelta(hours=100 + k * 7))
        speichere(b.filter(ImageFilter.GaussianBlur(3)), tmp_path / f"s{k}b.jpg", T0 + timedelta(hours=100 + k * 7, minutes=1))
    p = Projekt(tmp_path)
    p.analysieren()
    p.setze_einstellungen({"seiten_min": 4, "seiten_schritt": 2, "seiten_max": 100, "ereignisse_getrennt": False})
    a = p.ansicht()
    assert a["zahlen"]["serien"] == 2 and a["zahlen"]["im_buch"] == 10
    # 10 Fotos -> 4 Seiten, 12 Plaetze -> 2 fehlen; beide Serienreste passen
    assert a["seitenplan"]["fehlende_fotos"] == 2
    assert len(a["vorschlaege"]["auffuellen"]) == 2
    assert p.vorschlag_anwenden("auffuellen") == 2
    assert p.ansicht()["seitenplan"]["fehlende_fotos"] == 0


def test_export_kopiert_und_schreibt_seitenliste(projekt, tmp_path):
    ziel = tmp_path / "ausgabe"
    erg = projekt.exportieren(ziel)
    assert erg["fotos"] == 19 and erg["kopiert"] == 19 and not erg["fehler"]
    csv = (ziel / "seitenliste.csv").read_text(encoding="utf-8-sig").splitlines()
    assert csv[0].startswith("Seite;Ereignis") and len(csv) == 1 + 19 + (24 - 7 - 0) - (24 - 7) + 0 + _freie(projekt)
    ordner = sorted(p.name for p in ziel.iterdir() if p.is_dir())
    assert len(ordner) == 2 and ordner[0].startswith("01_")
    # Erneuter Export überschreibt nichts
    erg2 = projekt.exportieren(ziel)
    assert Path(erg2["ziel"]) != ziel and (ziel / "seitenliste.csv").exists()
    # Originale unverändert
    assert (tmp_path / "a03.jpg").exists()


def _freie(p):
    plan = p.seitenplan()
    return plan["buch_seiten"] - sum(plan["seiten_je_ereignis"])


def test_export_ordner_wird_nicht_als_fotoquelle_gelesen(projekt, tmp_path):
    projekt.exportieren()  # Standardziel im Fotoordner
    projekt.analysieren()
    assert projekt.ansicht()["zahlen"]["dateien"] == 21


def test_vorschau_liefert_jpeg_und_faellt_bei_defekten_dateien_zurueck(projekt, tmp_path):
    fid = next(iter(projekt.fotos))
    assert projekt.vorschau(fid, 200)[:2] == b"\xff\xd8"
    (tmp_path / "kaputt.jpg").write_bytes(b"kein bild")
    projekt.analysieren()
    kid = next(i for i, f in projekt.fotos.items() if f.name == "kaputt.jpg")
    assert projekt.fotos[kid].fehler
    assert projekt.vorschau(kid)[:2] == b"\xff\xd8"
    a = projekt.ansicht()
    assert a["zahlen"]["nicht_lesbar"] == 1 and not a["fotos"][kid]["b"]


def test_leerer_ordner(tmp_path):
    p = Projekt(tmp_path)
    p.analysieren()
    a = p.ansicht()
    assert a["analysiert"] is False and a["leer"] is True


# ---------------------------------------------------------------- Server

@pytest.fixture
def server(tmp_path):
    baue_ordner(tmp_path)
    app = App()
    httpd = starte_server(app)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    yield app, httpd, tmp_path
    httpd.shutdown()


def anfrage(httpd, pfad, token, daten=None, host=None, ctype="application/json"):
    req = urllib.request.Request(f"http://127.0.0.1:{httpd.server_address[1]}{pfad}")
    if token:
        req.add_header("X-Fotobuch-Token", token)
    if host:
        req.add_header("Host", host)
    if daten is not None:
        req.data = json.dumps(daten).encode()
        req.add_header("Content-Type", ctype)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def test_api_ohne_token_oder_fremder_host_wird_abgelehnt(server):
    app, httpd, _ = server
    assert anfrage(httpd, "/api/status", None)[0] == 403
    assert anfrage(httpd, "/api/status", "falsch")[0] == 403
    assert anfrage(httpd, "/api/status", app.token, host="boese.example")[0] == 403
    assert anfrage(httpd, "/api/ordner/oeffnen", None, {"pfad": "x"})[0] == 403
    assert anfrage(httpd, "/api/status", app.token)[0] == 200


def test_post_braucht_json(server):
    app, httpd, tmp = server
    code, _ = anfrage(httpd, "/api/ordner/oeffnen", app.token, {"pfad": str(tmp)}, ctype="text/plain")
    assert code == 415


def test_statische_dateien_ohne_ausbruch_aus_dem_webordner(server):
    app, httpd, _ = server
    code, body = anfrage(httpd, "/", None)
    assert code == 200 and app.token.encode() in body
    assert anfrage(httpd, "/app.js", None)[0] == 200
    assert anfrage(httpd, "/../projekt.py", None)[0] == 404
    assert anfrage(httpd, "/%2e%2e/projekt.py", None)[0] == 404


def test_ablauf_ueber_api(server):
    app, httpd, tmp = server
    t = app.token
    assert anfrage(httpd, "/api/ansicht", t)[0] == 400  # noch kein Ordner
    assert anfrage(httpd, "/api/ordner/oeffnen", t, {"pfad": str(tmp / "gibtsnicht")})[0] == 400
    assert anfrage(httpd, "/api/ordner/oeffnen", t, {"pfad": str(tmp)})[0] == 200
    assert anfrage(httpd, "/api/analyse/start", t, {})[0] == 200
    import time
    for _ in range(100):
        st = json.loads(anfrage(httpd, "/api/status", t)[1])
        if not st["job"]["laeuft"]:
            break
        time.sleep(0.1)
    assert st["job"]["phase"] == "fertig"
    a = json.loads(anfrage(httpd, "/api/ansicht", t)[1])
    assert a["zahlen"]["dateien"] == 21 and a["schritt"] == 2
    fid = next(iter(a["fotos"]))
    code, body = anfrage(httpd, f"/api/vorschau/{fid}?k=200&t={t}", None)
    assert code == 200 and body[:2] == b"\xff\xd8"
    assert anfrage(httpd, "/api/vorschau/../../etc/passwd", t)[0] in (400, 404)
    assert anfrage(httpd, "/api/vorschau/nichtda", t)[0] == 400
    assert anfrage(httpd, "/api/foto", t, {"id": fid, "aktion": "boese"})[0] == 400
    assert anfrage(httpd, "/api/foto", t, {"id": fid, "aktion": "entfernen"})[0] == 200
    code, body = anfrage(httpd, "/api/export", t, {"ziel": str(tmp / "exp")})
    assert code == 200 and json.loads(body)["fotos"] == 18


# ---------------------------------------------------------------- Orte

TIMES_SQUARE = (40.7580, -73.9855)
CENTRAL_PARK = (40.7739, -73.9712)


def _versatz(gps, meter_nord=0.0, meter_ost=0.0):
    return (gps[0] + meter_nord / 111_000, gps[1] + meter_ost / 84_000)


def baue_orte_ordner(d: Path) -> Path:
    # Times Square: 6 Fotos (2 ohne GPS von einer zweiten Kamera), später Central Park: 4 Fotos
    for i in range(4):
        speichere(motiv(10 + i), d / f"ts{i}.jpg", T0 + timedelta(minutes=i * 6), gps=_versatz(TIMES_SQUARE, i * 15, i * 10))
    for i in range(2):
        speichere(motiv(20 + i), d / f"dslr{i}.jpg", T0 + timedelta(minutes=i * 6 + 3), marke="Canon", modell="R50")
    for i in range(4):
        speichere(motiv(30 + i), d / f"cp{i}.jpg", T0 + timedelta(hours=2, minutes=i * 5), gps=_versatz(CENTRAL_PARK, i * 20, 0))
    return d


def test_orte_gruppieren_nach_gps_und_ordnen_kamera_ohne_gps_zu(tmp_path):
    p = Projekt(baue_orte_ordner(tmp_path))
    p.analysieren()
    a = p.ansicht()
    assert a["zahlen_orte"]["orte"] == 2 and a["zahlen_orte"]["ohne_ort"] == 0
    grosse = a["orte"][0]
    assert grosse["anzahl"] == 6 and grosse["mit_gps"] == 4  # inklusive 2 Fotos ohne GPS
    namen = {a["fotos"][i]["n"] for i in grosse["alle"]}
    assert {"dslr0.jpg", "dslr1.jpg", "ts0.jpg"} <= namen
    assert abs(grosse["lat"] - TIMES_SQUARE[0]) < 0.001
    # Rang: jedes Foto hat einen Rang 1..n im Ort, bestes zuerst
    raenge = sorted(a["fotos"][i]["r"] for i in grosse["alle"])
    assert raenge == [1, 2, 3, 4, 5, 6]


def test_ortsradius_bestimmt_die_gruppen(tmp_path):
    p = Projekt(baue_orte_ordner(tmp_path))
    p.analysieren()
    p.setze_einstellungen({"ort_radius_m": 25})
    assert p.ansicht()["zahlen_orte"]["orte"] > 2
    p.setze_einstellungen({"ort_radius_m": 5000})
    assert p.ansicht()["zahlen_orte"]["orte"] == 1


def test_max_pro_ort_behaelt_die_besten_und_ist_umkehrbar(tmp_path):
    p = Projekt(baue_orte_ordner(tmp_path))
    p.analysieren()
    a = p.ansicht()
    assert a["zahlen"]["im_buch"] == 10
    p.setze_einstellungen({"max_pro_ort": 2})
    b = p.ansicht()
    assert b["zahlen"]["im_buch"] == 4
    assert b["zahlen_orte"]["ort_entfernt"] == 6
    for o in b["orte"]:
        behalten = [i for i in o["alle"] if b["fotos"][i]["b"]]
        assert len(behalten) == 2
        assert set(behalten) == set(o["alle"][:2])  # genau die zwei bestplatzierten
    # Eigene Entscheidung gewinnt: ein entferntes Foto zurückholen
    weg = next(i for i, f in b["fotos"].items() if not f["b"])
    p.setze_foto(weg, "behalten")
    assert p.ansicht()["fotos"][weg]["b"]
    # Ausschalten
    p.setze_einstellungen({"max_pro_ort": ""})
    assert p.ansicht()["zahlen"]["im_buch"] == 10
    with pytest.raises(ProjektFehler):
        p.setze_einstellungen({"max_pro_ort": 0})


def test_serie_am_selben_ort_darf_laenger_dauern_an_anderen_orten_nie(tmp_path):
    basis = motiv(77)
    # selbes Motiv, 40 Minuten Abstand: ohne Ort keine Serie (Fenster 15 min)
    speichere(basis, tmp_path / "x1.jpg", T0, gps=TIMES_SQUARE)
    speichere(basis, tmp_path / "x2.jpg", T0 + timedelta(minutes=40), gps=_versatz(TIMES_SQUARE, 20, 0))
    # gleiches Motiv, aber 10 km entfernt aufgenommen: nie eine Serie
    speichere(basis, tmp_path / "y1.jpg", T0 + timedelta(days=1), gps=TIMES_SQUARE)
    speichere(basis, tmp_path / "y2.jpg", T0 + timedelta(days=1, minutes=2), gps=CENTRAL_PARK)
    for n in ("x2.jpg", "y2.jpg"):  # keine exakten Duplikate
        from PIL import ImageEnhance
        from PIL import Image
        bild = Image.open(tmp_path / n)
        exif = bild.getexif()
        ImageEnhance.Brightness(bild).enhance(1.02).save(tmp_path / n, exif=exif, quality=90)
    p = Projekt(tmp_path)
    p.analysieren()
    a = p.ansicht()
    namen = [sorted(a["fotos"][i]["n"] for i in s["ids"]) for s in a["serien"]]
    assert ["x1.jpg", "x2.jpg"] in namen
    assert ["y1.jpg", "y2.jpg"] not in namen


def test_export_enthaelt_ortsspalte(tmp_path):
    p = Projekt(baue_orte_ordner(tmp_path))
    p.analysieren()
    p.ort_umbenennen(p.ort_key(0), "Times Square")
    erg = p.exportieren(tmp_path / "exp")
    kopf, *zeilen = (Path(erg["ziel"]) / "seitenliste.csv").read_text(encoding="utf-8-sig").splitlines()
    assert kopf.endswith(";Ort") and any("Times Square" in z for z in zeilen)
