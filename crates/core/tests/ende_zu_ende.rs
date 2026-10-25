//! Ende-zu-Ende-Test des Kerns mit synthetischen Bildern (ohne EXIF: Zeit kommt aus dem Änderungsdatum).
use std::sync::{Arc, Mutex};

use fotobuch_core::projekt::{analysieren, Projekt};
use image::{Rgb, RgbImage};

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
    bild_mit_zeit(&dir.path().join(format!("a{minute:02}_pano.jpg")), 900, 300, 99, minute);

    let p = Arc::new(Mutex::new(Projekt::neu(dir.path()).unwrap()));
    analysieren(&p, &|_, _, _| {}).unwrap();
    let a = p.lock().unwrap().ansicht();
    let buch = &a["buch"];
    let seiten = buch["seiten"].as_array().unwrap();
    let ganz: Vec<_> = seiten.iter().filter(|s| s["ganz"] == true).collect();
    assert_eq!(ganz.len(), 1, "genau das Panorama hat eine eigene Seite");
    assert_eq!(ganz[0]["fotos"].as_array().unwrap().len(), 1);
    assert_eq!(seiten.len() as i64, a["seitenplan"]["seiten_benoetigt"].as_i64().unwrap(), "Seitenzähler stimmt mit den Seiten überein");

    // Gemischte Seiten: ohne Paarung wären alle Seiten rein hoch oder rein quer
    let ar = buch["ar"].as_f64().unwrap();
    let gemischt = seiten
        .iter()
        .filter(|s| s["ganz"] == false)
        .filter(|s| {
            let formen: Vec<bool> = s["fotos"].as_array().unwrap().iter().map(|f| f["w"].as_f64().unwrap() * ar > f["h"].as_f64().unwrap()).collect();
            formen.contains(&true) && formen.contains(&false)
        })
        .count();
    assert!(gemischt >= 2, "Hoch- und Querformate teilen sich Seiten: {gemischt}");

    // Das Panorama lässt sich als normales Foto behandeln und zurück
    let id = ganz[0]["fotos"][0]["id"].as_str().unwrap().to_string();
    p.lock().unwrap().setze_ganzseite(&id, Some(false)).unwrap();
    let a2 = p.lock().unwrap().ansicht();
    assert!(a2["buch"]["seiten"].as_array().unwrap().iter().all(|s| s["ganz"] == false));
    p.lock().unwrap().setze_ganzseite(&id, None).unwrap();
    assert_eq!(p.lock().unwrap().ansicht()["buch"]["seiten"].as_array().unwrap().iter().filter(|s| s["ganz"] == true).count(), 1);
}
