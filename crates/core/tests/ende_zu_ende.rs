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
