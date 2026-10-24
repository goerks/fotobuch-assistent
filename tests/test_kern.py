import json
import shutil
from datetime import datetime, timedelta

import pytest
from PIL import Image, ImageFilter

from conftest import motiv, speichere
from fotobuch.duplicates import finde_aehnliche, finde_exakte
from fotobuch.events import bilde_ereignisse
from fotobuch.pages import Profil, erlaubte_seitenzahl, lade_profil, seitenplan
from fotobuch.report import Optionen, analysiere_ordner, exportiere_sortiert, schreibe_berichte, zusammenfassung
from fotobuch.scanner import analysiere_foto, finde_fotos, scanne

T0 = datetime(2026, 7, 3, 10, 0, 0)


def test_exif_zeit_und_kamera(tmp_path):
    p = speichere(motiv(1), tmp_path / "a.jpg", T0)
    f = analysiere_foto(p)
    assert f.zeit == T0
    assert f.zeit_quelle == "exif"
    assert "TestKamera" in f.kamera
    assert (f.breite, f.hoehe) == (640, 480)
    assert f.format == "quer"
    assert not f.fehler


def test_hochformat_per_exif_drehung(tmp_path):
    p = speichere(motiv(1), tmp_path / "h.jpg", T0, orientierung=6)
    f = analysiere_foto(p)
    assert f.format == "hoch"
    assert (f.breite, f.hoehe) == (480, 640)


def test_ohne_exif_dateidatum_und_hinweis(tmp_path):
    p = tmp_path / "x.jpg"
    motiv(2).save(p)
    f = analysiere_foto(p)
    assert f.zeit_quelle == "datei"
    assert any("Dateidatum" in h for h in f.hinweise)


def test_kaputte_datei_bricht_nicht_ab(tmp_path):
    (tmp_path / "kaputt.jpg").write_bytes(b"das ist kein bild")
    f = analysiere_foto(tmp_path / "kaputt.jpg")
    assert f.fehler


def test_screenshot_verdacht(tmp_path):
    p = tmp_path / "Screenshot 2026.png"
    motiv(3).save(p)
    assert analysiere_foto(p).screenshot_verdacht
    q = tmp_path / "bild.png"
    motiv(4).save(q)
    assert analysiere_foto(q).screenshot_verdacht  # PNG ohne Kamera


def test_heic_wird_gelesen(tmp_path):
    pillow_heif = pytest.importorskip("pillow_heif")
    p = tmp_path / "iphone.heic"
    try:
        speichere(motiv(5), p, T0, format="HEIF")
    except Exception as e:  # Encoder nicht in jeder Installation
        pytest.skip(f"HEIC-Schreiben nicht möglich: {e}")
    f = analysiere_foto(p)
    assert not f.fehler, f.fehler
    assert f.zeit == T0


def test_ereignisse_nach_zeitluecke(tmp_path):
    zeiten = [T0, T0 + timedelta(minutes=30), T0 + timedelta(hours=1),
              T0 + timedelta(days=1), T0 + timedelta(days=1, minutes=10)]
    for i, z in enumerate(zeiten):
        speichere(motiv(10 + i), tmp_path / f"f{i}.jpg", z)
    fotos = scanne(finde_fotos(tmp_path))
    ev = bilde_ereignisse(fotos, luecke_stunden=6)
    assert [len(e.fotos) for e in ev] == [3, 2]
    assert ev[0].titel.startswith("Fr 03.07.2026")


def test_sortierung_mehrerer_geraete(tmp_path):
    speichere(motiv(1), tmp_path / "zzz.jpg", T0 + timedelta(minutes=5), modell="Handy")
    speichere(motiv(2), tmp_path / "aaa.jpg", T0 + timedelta(minutes=10), modell="Kamera")
    speichere(motiv(3), tmp_path / "mmm.jpg", T0, modell="Kamera")
    ev = bilde_ereignisse(scanne(finde_fotos(tmp_path)))
    assert [f.name for f in ev[0].fotos] == ["mmm.jpg", "zzz.jpg", "aaa.jpg"]


def test_exakte_duplikate(tmp_path):
    p1 = speichere(motiv(1), tmp_path / "a.jpg", T0)
    shutil.copy(p1, tmp_path / "a_kopie.jpg")
    speichere(motiv(2), tmp_path / "b.jpg", T0 + timedelta(minutes=1))
    fotos = scanne(finde_fotos(tmp_path))
    gruppen = finde_exakte(fotos)
    assert len(gruppen) == 1
    assert {f.name for f in gruppen[0].fotos} == {"a.jpg", "a_kopie.jpg"}


def test_serie_gefunden_und_schaerfstes_vorgeschlagen(tmp_path):
    basis = motiv(7)
    speichere(basis.filter(ImageFilter.GaussianBlur(3)), tmp_path / "s1_unscharf.jpg", T0)
    speichere(basis, tmp_path / "s2_scharf.jpg", T0 + timedelta(seconds=3))
    speichere(motiv(99), tmp_path / "anderes.jpg", T0 + timedelta(seconds=6))
    fotos = scanne(finde_fotos(tmp_path))
    serien = finde_aehnliche(fotos)
    assert len(serien) == 1
    assert {f.name for f in serien[0].fotos} == {"s1_unscharf.jpg", "s2_scharf.jpg"}
    assert serien[0].vorschlag.name == "s2_scharf.jpg"


def test_aehnlich_aber_zeitlich_weit_weg_ist_keine_serie(tmp_path):
    basis = motiv(7)
    speichere(basis, tmp_path / "a.jpg", T0)
    speichere(basis, tmp_path / "b.jpg", T0 + timedelta(hours=5))
    # unterschiedliche Datei-Bytes durch andere Qualität
    speichere(basis, tmp_path / "c.jpg", T0 + timedelta(hours=5, minutes=1), quality=70)
    serien = finde_aehnliche(scanne(finde_fotos(tmp_path)), fenster_sekunden=900)
    namen = [sorted(f.name for f in g.fotos) for g in serien]
    assert ["a.jpg", "b.jpg"] not in namen


def test_seitenzahl_stufen():
    p = Profil("t", False, 24, 2, 100)
    assert erlaubte_seitenzahl(10, p) == 24
    assert erlaubte_seitenzahl(24, p) == 24
    assert erlaubte_seitenzahl(25, p) == 26
    p4 = Profil("t4", False, 24, 4, 100)
    assert erlaubte_seitenzahl(25, p4) == 28


def test_seitenplan_fehlende_fotos():
    p = Profil("t", False, 24, 2, 100)
    r = seitenplan(70, 3, p)  # 24 Seiten benötigt, Buch 24, 72 Fotos füllen es
    assert r["buch_seiten"] == 24
    assert r["fehlende_fotos"] == 2
    r2 = seitenplan(75, 3, p)  # 25 Seiten benötigt, Buch 26
    assert r2["buch_seiten"] == 26
    assert r2["fehlende_fotos"] == 3
    assert r2["ueberzaehlige_fotos_fuer_kleinere_stufe"] == 3  # 75 - 24*3


def test_profile_laden():
    for name in ("allgemein", "cewe-pixum", "ifolor"):
        pr = lade_profil(name).mit_vorgaben()
        assert pr.seiten_min and pr.seiten_schritt
    assert not lade_profil("cewe-pixum").verifiziert


def test_ende_zu_ende_und_teilbarer_bericht(tmp_path):
    quelle = tmp_path / "Urlaub-Geheim"
    quelle.mkdir()
    basis = motiv(7)
    speichere(basis, quelle / "IMG_secret_1.jpg", T0)
    speichere(basis.filter(ImageFilter.GaussianBlur(2)), quelle / "IMG_secret_2.jpg", T0 + timedelta(seconds=2))
    shutil.copy(quelle / "IMG_secret_1.jpg", quelle / "IMG_secret_1_kopie.jpg")
    speichere(motiv(8), quelle / "IMG_secret_3.jpg", T0 + timedelta(days=1))
    motiv(9).save(quelle / "Screenshot.png")

    opt = Optionen(profil=lade_profil("allgemein").mit_vorgaben(), fotos_pro_seite=3)
    erg = analysiere_ordner(quelle, opt)
    zus = zusammenfassung(erg)
    assert zus["anzahl_dateien"] == 5
    assert zus["exakte_duplikat_ueberzaehlig"] == 1
    assert zus["serien_gruppen"] == 1
    assert zus["screenshot_verdacht"] == 1
    assert zus["varianten_fotozahl"]["ohne_duplikate_serien_und_screenshots"] == 2

    ziel = tmp_path / "bericht"
    dateien = schreibe_berichte(erg, ziel)
    teil = (ziel / "bericht-teilbar.json").read_text(encoding="utf-8")
    assert "secret" not in teil and "Urlaub-Geheim" not in teil
    assert str(tmp_path) not in teil
    daten = json.loads(teil)
    assert daten["hardware"]["kerne"]
    assert "reihenfolge" in dateien
    assert "IMG_secret_1.jpg" in (ziel / "reihenfolge.csv").read_text(encoding="utf-8-sig")


def test_export_kopiert_und_laesst_originale(tmp_path):
    quelle = tmp_path / "q"
    quelle.mkdir()
    speichere(motiv(1), quelle / "a.jpg", T0)
    shutil.copy(quelle / "a.jpg", quelle / "a2.jpg")
    speichere(motiv(2), quelle / "b.jpg", T0 + timedelta(days=2))
    erg = analysiere_ordner(quelle, Optionen(profil=lade_profil("allgemein").mit_vorgaben()))
    ziel = tmp_path / "z"
    n = exportiere_sortiert(erg, ziel)
    assert n == 2
    assert len(list(quelle.glob("*.jpg"))) == 3
    assert sorted(p.name for p in ziel.iterdir()) == ["01_2026-07-03", "02_2026-07-05"]


def test_lange_kette_ergibt_keine_riesenserie(tmp_path):
    """Gleiches Motiv alle 7 Minuten über 2 Stunden: Serien bleiben zeitlich begrenzt."""
    basis = motiv(7)
    for i in range(18):
        speichere(basis, tmp_path / f"k{i:02d}.jpg", T0 + timedelta(minutes=i * 7), quality=60 + i)
    fotos = scanne(finde_fotos(tmp_path))
    serien = finde_aehnliche(fotos, fenster_sekunden=900)
    assert len(serien) > 1
    for g in serien:
        spanne = (g.fotos[-1].zeit - g.fotos[0].zeit).total_seconds()
        assert spanne <= 1800
