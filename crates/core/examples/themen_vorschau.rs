//! Entwicklerwerkzeug: Zeichnet Deckblatt und die ersten Seiten in allen Themen (verkleinert) in einen Ordner.
//! Aufruf: cargo run --release --example themen_vorschau -- <Fotoordner> <Ausgabeordner>
use std::sync::{Arc, Mutex};

use fotobuch_core::projekt::{analysieren, Projekt};
use fotobuch_core::{gestaltung::THEMEN, seitenexport, thema_render};
use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (quelle, ziel) = (&args[1], std::path::PathBuf::from(&args[2]));
    std::fs::create_dir_all(&ziel).unwrap();
    let p = Arc::new(Mutex::new(Projekt::neu(std::path::Path::new(quelle)).unwrap()));
    analysieren(&p, &|_, _, _| {}).unwrap();
    for t in THEMEN.iter() {
        let paket = {
            let mut pj = p.lock().unwrap();
            pj.setze_gestaltung(json!({"thema": t.id, "titel": "New York 2026", "untertitel": "", "name": "Familie Muster"}).as_object().unwrap()).unwrap();
            pj.seitenauftraege().unwrap()
        };
        let (b, h) = (1169, 877);
        let d = &paket.deckblatt;
        thema_render::rendere_deckblatt(&paket.stil, b, h, &d.titel, &d.untertitel, &d.name, d.foto.as_ref()).save(ziel.join(format!("{}-0-deckblatt.jpg", t.id))).unwrap();
        for (k, a) in paket.auftraege.iter().take(4).enumerate() {
            seitenexport::rendere_seite(a, b, h, &paket.stil).save(ziel.join(format!("{}-{}.jpg", t.id, k + 1))).unwrap();
        }
    }
}
