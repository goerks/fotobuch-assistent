//! Ende-zu-Ende-Test des Kerns mit synthetischen Bildern (ohne EXIF: Zeit kommt aus dem Änderungsdatum).
use std::sync::{Arc, Mutex};

use fotobuch_core::projekt::{analysieren, Projekt};
use image::{Rgb, RgbImage};
use serde_json::{json, Value};

fn motiv(seed: u32) -> RgbImage {
    RgbImage::from_fn(320, 240, |x, y| {
        let v = ((x * (seed + 3) + y * (seed + 7)) % 255) as u8;
        Rgb([v, ((x / 8 + seed) * 17 % 255) as u8, ((y / 8 + seed * 3) * 13 % 255) as u8])
    })
}

#[test]
fn analyse_findet_duplikate_und_speichert_entscheidungen() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        motiv(i * 11).save(dir.path().join(format!("a{i}.jpg"))).unwrap();
    }
    std::fs::copy(dir.path().join("a1.jpg"), dir.path().join("a1_kopie.jpg")).unwrap();
    std::fs::write(dir.path().join("kaputt.jpg"), b"kein bild").unwrap();

    let p = Arc::new(Mutex::new(Projekt::neu(dir.path()).unwrap()));
    analysieren(&p, &|_, _, _| {}).unwrap();
    let a = p.lock().unwrap().ansicht();
    assert_eq!(a["zahlen"]["dateien"], 7);
    assert_eq!(a["zahlen"]["nicht_lesbar"], 1);
    assert_eq!(a["zahlen"]["exakte_gruppen"], 1);
    let vorher = a["zahlen"]["im_buch"].as_i64().unwrap();
    assert!(vorher <= 5, "mindestens das exakte Duplikat ist aussortiert");

    // Entscheidung überlebt einen Neustart
    let id = a["fotos"].as_object().unwrap().iter().find(|(_, v)| v["n"] == "a0.jpg" || v["b"] == true).unwrap().0.clone();
    p.lock().unwrap().setze_foto(&id, "entfernen").unwrap();
    drop(p);
    let p2 = Projekt::neu(dir.path()).unwrap();
    assert_eq!(p2.ansicht()["zahlen"]["im_buch"].as_i64().unwrap(), vorher - 1);
    assert!(p2.setze_foto_gueltig("nix").is_err());
}

fn bild_mit_zeit(pfad: &std::path::Path, breite: u32, hoehe: u32, seed: u32, minute: u64) {
    // Blockmuster aus einer Streuzahl je Block: jedes Bild sieht anders aus, es entstehen keine Serien
    RgbImage::from_fn(breite, hoehe, |x, y| {
        let mut h = (x / 20 + 1000 * (y / 20) + 77_777 * seed).wrapping_mul(2_654_435_761);
        h ^= h >> 15;
        h = h.wrapping_mul(2_246_822_519);
        h ^= h >> 13;
        Rgb([(h & 255) as u8, ((h >> 8) & 255) as u8, ((h >> 16) & 255) as u8])
    })
    .save(pfad)
    .unwrap();
    let f = std::fs::OpenOptions::new().write(true).open(pfad).unwrap();
    f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000 + minute * 60)).unwrap();
}

/// Hoch- und Querformate wechseln blockweise, dazu ein Panorama: Seiten werden gemischt gefüllt, das Panorama bekommt eine eigene Seite.
#[test]
fn seitenplanung_mit_panorama_und_hoch_quer_paarung() {
    let dir = tempfile::tempdir().unwrap();
    let mut minute = 0;
    // Aufnahmezeit: EXIF fehlt, deshalb kommt die Zeit aus dem Änderungsdatum
    for block in 0..2u32 {
        for k in 0..3u32 {
            minute += 1;
            bild_mit_zeit(&dir.path().join(format!("a{minute:02}_hoch.jpg")), 300, 400, block * 10 + k + 1, minute);
        }
        for k in 0..3u32 {
            minute += 1;
            bild_mit_zeit(&dir.path().join(format!("a{minute:02}_quer.jpg")), 400, 300, block * 10 + k + 51, minute);
        }
    }
    minute += 1;
    bild_mit_zeit(&dir.path().join(format!("a{minute:02}_pano.jpg")), 900, 300, 99, minute); // 3:1, über die Doppelseite
    minute += 1;
    bild_mit_zeit(&dir.path().join(format!("a{minute:02}_breit.jpg")), 720, 300, 98, minute); // 2.4:1, eigene Seite

    let p = Arc::new(Mutex::new(Projekt::neu(dir.path()).unwrap()));
    analysieren(&p, &|_, _, _| {}).unwrap();
    let a = p.lock().unwrap().ansicht();
    let buch = &a["buch"];
    let seiten = buch["seiten"].as_array().unwrap();
    let art = |s: &Value| s["art"].as_i64().unwrap();
    assert_eq!(seiten.iter().filter(|s| art(s) == 1).count(), 1, "das 2.4:1-Foto hat eine eigene Seite");
    let links = seiten.iter().position(|s| art(s) == 2).expect("Panorama über die Doppelseite");
    assert_eq!(art(&seiten[links + 1]), 3);
    assert_eq!(seiten[links]["fotos"][0]["id"], seiten[links + 1]["fotos"][0]["id"], "beide Hälften zeigen dasselbe Foto");
    assert_eq!((links + 1) % 2, 0, "die linke Hälfte hat eine gerade Seitennummer");
    assert!(seiten[links]["fotos"][0]["w"].as_f64().unwrap() > 100.0, "das Foto reicht über den Seitenrand hinaus");
    assert_eq!(seiten.len() as i64, a["seitenplan"]["seiten_benoetigt"].as_i64().unwrap(), "Seitenzähler stimmt mit den Seiten überein");

    // Gemischte Seiten: ohne Paarung wären alle Normalseiten rein hoch oder rein quer
    let ar = buch["ar"].as_f64().unwrap();
    let gemischt = seiten
        .iter()
        .filter(|s| art(s) == 0)
        .filter(|s| {
            let formen: Vec<bool> = s["fotos"].as_array().unwrap().iter().map(|f| f["w"].as_f64().unwrap() * ar > f["h"].as_f64().unwrap()).collect();
            formen.contains(&true) && formen.contains(&false)
        })
        .count();
    assert!(gemischt >= 2, "Hoch- und Querformate teilen sich Seiten: {gemischt}");

    // Seitenart umstellen und zurück
    let id = seiten[links]["fotos"][0]["id"].as_str().unwrap().to_string();
    p.lock().unwrap().setze_seitenart(&id, Some("normal")).unwrap();
    let a2 = p.lock().unwrap().ansicht();
    assert!(a2["buch"]["seiten"].as_array().unwrap().iter().all(|s| art(s) != 2 && art(s) != 3));
    p.lock().unwrap().setze_seitenart(&id, None).unwrap();
    assert!(p.lock().unwrap().ansicht()["buch"]["seiten"].as_array().unwrap().iter().any(|s| art(s) == 2));

    // Foto von Hand auf eine andere Seite legen und zurück
    let (erste, zweite) = (&seiten[0], &seiten[1]);
    let foto = erste["fotos"][0]["id"].as_str().unwrap().to_string();
    let (n1, n2) = (erste["fotos"].as_array().unwrap().len(), zweite["fotos"].as_array().unwrap().len());
    p.lock().unwrap().verschiebe_foto(&foto, Some(2)).unwrap();
    let a3 = p.lock().unwrap().ansicht();
    let s3 = a3["buch"]["seiten"].as_array().unwrap();
    assert_eq!(s3[0]["fotos"].as_array().unwrap().len(), n1 - 1);
    assert_eq!(s3[1]["fotos"].as_array().unwrap().len(), n2 + 1);
    assert!(s3[1]["fotos"].as_array().unwrap().iter().any(|f| f["id"] == json!(foto)));
    assert_eq!(a3["fotos"][&foto]["vs"], true);
    assert!(p.lock().unwrap().verschiebe_foto(&foto, Some(9999)).is_err());
    let doppel_nr = links + 1;
    assert!(p.lock().unwrap().verschiebe_foto(&foto, Some(doppel_nr)).is_err(), "auf eine Seite mit Ganzseiten-Foto geht es nicht");
    p.lock().unwrap().verschiebe_foto(&foto, None).unwrap();
    let a4 = p.lock().unwrap().ansicht();
    assert_eq!(a4["buch"]["seiten"][0]["fotos"].as_array().unwrap().len(), n1);
    assert_eq!(a4["fotos"][&foto]["vs"], false);

    // Seitenfolge: leere Seite einfügen, Seite verschieben, entfernen, alles zurücknehmen
    let key = |a: &Value, i: usize| a["buch"]["seiten"][i]["key"].as_str().unwrap().to_string();
    let n0 = a4["buch"]["seiten"].as_array().unwrap().len();
    let (k0, k1, k2) = (key(&a4, 0), key(&a4, 1), key(&a4, 2));
    p.lock().unwrap().leere_seite_einfuegen(&k1, false).unwrap();
    let a5 = p.lock().unwrap().ansicht();
    assert_eq!(a5["buch"]["seiten"].as_array().unwrap().len(), n0 + 1);
    assert_eq!(art(&a5["buch"]["seiten"][1]), 4, "leere Seite steht vor Seite 2");
    assert!(a5["buch"]["seiten"][1]["fotos"].as_array().unwrap().is_empty());
    assert_eq!(a5["seitenplan"]["eingefuegte_leere"], 1);
    assert!(a5["seitenplan"]["seiten_benoetigt"].as_i64().unwrap() >= n0 as i64 + 1);
    // Seite 1 hinter Seite 3 stellen
    p.lock().unwrap().seite_verschieben(&k0, &k2, true).unwrap();
    let a6 = p.lock().unwrap().ansicht();
    assert_eq!(key(&a6, 3), k0, "Seite 1 steht jetzt nach der ursprünglichen Seite 3");
    // leere Seite wieder entfernen
    let lk = a6["buch"]["seiten"].as_array().unwrap().iter().find(|s| art(s) == 4).unwrap()["key"].as_str().unwrap().to_string();
    p.lock().unwrap().seite_entfernen(&lk).unwrap();
    assert!(p.lock().unwrap().ansicht()["buch"]["seiten"].as_array().unwrap().iter().all(|s| art(s) != 4));
    // normale Seite entfernen: Fotos wandern auf die Nachbarseite
    let a7 = p.lock().unwrap().ansicht();
    let fotos_vor: usize = a7["buch"]["seiten"].as_array().unwrap().iter().map(|s| s["fotos"].as_array().unwrap().len()).sum();
    let nr_normal = a7["buch"]["seiten"].as_array().unwrap().iter().position(|s| art(s) == 0).unwrap();
    let nk = key(&a7, nr_normal);
    p.lock().unwrap().seite_entfernen(&nk).unwrap();
    let a8 = p.lock().unwrap().ansicht();
    let fotos_nach: usize = a8["buch"]["seiten"].as_array().unwrap().iter().map(|s| s["fotos"].as_array().unwrap().len()).sum();
    assert_eq!(fotos_vor, fotos_nach, "kein Foto geht verloren");
    assert_eq!(a8["buch"]["seiten"].as_array().unwrap().len(), n0 - 1, "eine Seite weniger");
    p.lock().unwrap().seitenfolge_zuruecksetzen();
    let a9 = p.lock().unwrap().ansicht();
    assert_eq!(a9["buch"]["seiten"].as_array().unwrap().len(), n0);
    assert_eq!(key(&a9, 0), k0);
    // Seitenaufträge enthalten leere Seiten
    p.lock().unwrap().leere_seite_einfuegen(&k0, true).unwrap();
    let paket = p.lock().unwrap().seitenauftraege().unwrap();
    assert!(paket.auftraege.iter().any(|a| a.fotos.is_empty()));
}

/// Gestaltung: Thema wählen, Wahl bleibt nach Neustart, Felder liegen im Themenbereich, Export schreibt Deckblatt und Seiten im Thema.
#[test]
fn gestaltung_thema_wahl_bleibt_und_export_schreibt_deckblatt() {
    let basis = tempfile::tempdir().unwrap();
    let dir = basis.path().join("Meine_Reise-2026");
    std::fs::create_dir(&dir).unwrap();
    for k in 0..6u32 {
        // zwei Tage: Zeit aus dem Änderungsdatum
        bild_mit_zeit(&dir.join(format!("f{k}.jpg")), 400, 300, k + 5, if k < 3 { k as u64 } else { 1440 * 2 + k as u64 });
    }
    let p = Arc::new(Mutex::new(Projekt::neu(&dir).unwrap()));
    analysieren(&p, &|_, _, _| {}).unwrap();
    let a = p.lock().unwrap().ansicht();
    let g = &a["gestaltung"];
    assert_eq!(g["wahl"]["thema"], "klar", "Standard");
    assert_eq!(g["themen"].as_array().unwrap().len(), 6);
    assert_eq!(g["titel_auto"], "Meine Reise 2026", "Titel aus dem Ordnernamen");
    assert_eq!(g["untertitel_auto"], "", "ohne EXIF-Aufnahmezeit gibt es keinen Zeitraum (das Änderungsdatum ist unzuverlässig)");
    let klar_feld = a["buch"]["seiten"][0]["fotos"][0].clone();

    {
        let mut pj = p.lock().unwrap();
        assert!(pj.setze_gestaltung(json!({"thema": "gibtsnicht"}).as_object().unwrap()).is_err());
        assert!(pj.setze_gestaltung(json!({"titelfoto": "unbekannt"}).as_object().unwrap()).is_err());
        pj.setze_gestaltung(json!({"thema": "galerie", "titel": "Unsere Reise", "name": "Familie Muster", "farbton": "#00AA55"}).as_object().unwrap()).unwrap();
    }
    let a2 = p.lock().unwrap().ansicht();
    assert_eq!(a2["gestaltung"]["stil"]["rahmen"], "linie");
    assert_eq!(a2["gestaltung"]["stil"]["akzent"], "#00aa55");
    let feld = &a2["buch"]["seiten"][0]["fotos"][0];
    assert!(feld["x"].as_f64().unwrap() > klar_feld["x"].as_f64().unwrap(), "Galerie lässt mehr Rand: {feld} gegen {klar_feld}");
    assert!(feld["w"].as_f64().unwrap() < klar_feld["w"].as_f64().unwrap());

    // Wahl überlebt einen Neustart
    drop(p);
    let p2 = Arc::new(Mutex::new(Projekt::neu(&dir).unwrap()));
    assert_eq!(p2.lock().unwrap().ansicht()["gestaltung"]["wahl"]["name"], "Familie Muster");

    // Export: Deckblatt mit den gewählten Texten, Seiten mit dem Hintergrund des Themas
    let paket = p2.lock().unwrap().seitenauftraege().unwrap();
    assert_eq!(paket.deckblatt.titel, "Unsere Reise");
    assert!(paket.deckblatt.foto.is_some(), "erstes Foto im Buch als Titelfoto");
    let ziel = basis.path().join("aus");
    fotobuch_core::seitenexport::exportiere(&paket.auftraege, Some(&paket.deckblatt), paket.leere, "quadrat", &paket.stil, &ziel, &|_, _| {}).unwrap();
    assert!(ziel.join("deckblatt.jpg").exists() && ziel.join("deckblatt.pdf").exists() && ziel.join("fotobuch.pdf").exists());
    let s1 = image::open(ziel.join("seite-001.jpg")).unwrap().to_rgb8();
    let ecke = s1.get_pixel(5, 5).0;
    assert!(ecke.iter().zip([247u8, 247, 245]).all(|(a, b)| (*a as i32 - b as i32).abs() < 8), "Galerie-Hintergrund: {ecke:?}");
    let c = image::open(ziel.join("deckblatt.jpg")).unwrap().to_rgb8();
    assert_eq!(c.dimensions(), (2480, 2480));
}
