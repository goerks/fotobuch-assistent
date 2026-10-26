//! Fertige Seiten: Jede Buchseite wird als JPEG (300 dpi) gerendert, dazu eine PDF mit allen Seiten.
//! Die Fotos werden nur gelesen, Originale bleiben unverändert.

use std::io::Write;
use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::{Rgb, RgbImage};
use rayon::prelude::*;
use serde_json::{json, Value};

use crate::scanner::{drehe, lade_bild, orientierung_von};

type Res<T> = Result<T, String>;

pub const DPI: f64 = 300.0;

/// Ein Foto auf einer Seite: Datei, Endung und Platz (x, y, Breite, Höhe in Prozent der Seite).
pub struct SeitenFoto {
    pub pfad: PathBuf,
    pub endung: String,
    pub feld: [f64; 4],
}

pub struct Auftrag {
    pub nr: usize,
    pub fotos: Vec<SeitenFoto>,
}

/// Seitengrösse in Pixeln bei 300 dpi (lange Kante 297 mm, Quadrat 210 mm).
pub fn seitengroesse(format: &str) -> (u32, u32) {
    match format {
        "quadrat" => (2480, 2480),
        "hoch" => (2631, 3508),
        _ => (3508, 2631),
    }
}

/// Rendert eine Seite: weisser Grund, jedes Foto füllt sein Feld (bei abweichendem Seitenverhältnis wird mittig beschnitten).
pub fn rendere_seite(a: &Auftrag, breite: u32, hoehe: u32) -> RgbImage {
    let mut seite = RgbImage::from_pixel(breite, hoehe, Rgb([255, 255, 255]));
    for f in &a.fotos {
        let px = |p: f64, g: u32| (p / 100.0 * g as f64).round() as i64;
        let (x, y) = (px(f.feld[0], breite), px(f.feld[1], hoehe));
        let (w, h) = (px(f.feld[2], breite).max(1) as u32, px(f.feld[3], hoehe).max(1) as u32);
        let bild = std::fs::read(&f.pfad).map_err(|e| e.to_string()).and_then(|daten| {
            let g = lade_bild(&f.pfad, &daten, &f.endung, (w.max(h) as f64 * 1.5) as u32)?;
            let o = orientierung_von(&g);
            Ok(drehe(g.rgb, o))
        });
        match bild {
            Ok(img) => {
                let (iw, ih) = (img.width() as f64, img.height() as f64);
                let s = (w as f64 / iw).max(h as f64 / ih);
                let (nw, nh) = (((iw * s).ceil() as u32).max(w), ((ih * s).ceil() as u32).max(h));
                let gross = imageops::resize(&img, nw, nh, FilterType::CatmullRom);
                let zugeschnitten = imageops::crop_imm(&gross, (nw - w) / 2, (nh - h) / 2, w, h).to_image();
                imageops::replace(&mut seite, &zugeschnitten, x, y);
            }
            Err(_) => {
                // Foto nicht lesbar: grauer Platzhalter statt Abbruch
                let grau = RgbImage::from_pixel(w, h, Rgb([210, 213, 218]));
                imageops::replace(&mut seite, &grau, x, y);
            }
        }
    }
    seite
}

fn jpeg_bytes(img: &RgbImage, qualitaet: u8) -> Res<Vec<u8>> {
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, qualitaet);
    img.write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Schreibt eine PDF mit einer Seite pro JPEG-Datei (Bild füllt die Seite, Seitengrösse aus Pixeln und 300 dpi).
#[allow(unused_assignments)]
pub fn schreibe_pdf(ziel: &Path, jpegs: &[PathBuf], breite: u32, hoehe: u32) -> Res<()> {
    let datei = std::fs::File::create(ziel).map_err(|e| format!("PDF kann nicht geschrieben werden: {e}"))?;
    let mut w = std::io::BufWriter::new(datei);
    let mut pos: usize = 0;
    let mut offsets: Vec<usize> = Vec::new(); // Objekt n steht an offsets[n-1]
    let (bpt, hpt) = (breite as f64 * 72.0 / DPI, hoehe as f64 * 72.0 / DPI);
    let n = jpegs.len();
    macro_rules! schreibe {
        ($bytes:expr) => {{
            let tmp = $bytes;
            let b: &[u8] = tmp.as_ref();
            w.write_all(b).map_err(|e| e.to_string())?;
            pos += b.len();
        }};
    }
    schreibe!(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    // Objekte: 1 Katalog, 2 Seitenbaum, danach je Seite (Seite, Bild, Inhalt)
    offsets.push(pos);
    schreibe!(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pos);
    let kids: Vec<String> = (0..n).map(|k| format!("{} 0 R", 3 + 3 * k)).collect();
    schreibe!(format!("2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n", kids.join(" "), n));
    for (k, pfad) in jpegs.iter().enumerate() {
        let (seite, bild, inhalt) = (3 + 3 * k, 4 + 3 * k, 5 + 3 * k);
        offsets.push(pos);
        schreibe!(format!(
            "{seite} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {bpt:.2} {hpt:.2}] /Resources << /XObject << /Im0 {bild} 0 R >> >> /Contents {inhalt} 0 R >>\nendobj\n"
        ));
        let daten = std::fs::read(pfad).map_err(|e| e.to_string())?;
        offsets.push(pos);
        schreibe!(format!(
            "{bild} 0 obj\n<< /Type /XObject /Subtype /Image /Width {breite} /Height {hoehe} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            daten.len()
        ));
        schreibe!(&daten);
        schreibe!(b"\nendstream\nendobj\n");
        let strom = format!("q {bpt:.2} 0 0 {hpt:.2} 0 0 cm /Im0 Do Q");
        offsets.push(pos);
        schreibe!(format!("{inhalt} 0 obj\n<< /Length {} >>\nstream\n{strom}\nendstream\nendobj\n", strom.len()));
    }
    let xref = pos;
    schreibe!(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1));
    for o in &offsets {
        schreibe!(format!("{o:010} 00000 n \n"));
    }
    schreibe!(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1));
    w.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Rendert alle Seiten (parallel) nach `ordner/seite-001.jpg …` und schreibt `ordner/fotobuch.pdf`.
/// `leere`: Anzahl freier Seiten am Ende, die als weisse Seiten mitgehen.
pub fn exportiere(auftraege: &[Auftrag], leere: usize, format: &str, ordner: &Path, fortschritt: &(dyn Fn(usize, usize) + Sync)) -> Res<Value> {
    std::fs::create_dir_all(ordner).map_err(|e| format!("Zielordner kann nicht angelegt werden: {e}"))?;
    let (b, h) = seitengroesse(format);
    let gesamt = auftraege.len() + leere;
    let fertig = std::sync::atomic::AtomicUsize::new(0);
    fortschritt(0, gesamt);
    let pfade: Vec<PathBuf> = (1..=gesamt).map(|nr| ordner.join(format!("seite-{nr:03}.jpg"))).collect();
    let ergebnisse: Vec<Res<()>> = (0..gesamt)
        .into_par_iter()
        .map(|k| {
            let img = if k < auftraege.len() { rendere_seite(&auftraege[k], b, h) } else { RgbImage::from_pixel(b, h, Rgb([255, 255, 255])) };
            let bytes = jpeg_bytes(&img, 92)?;
            std::fs::write(&pfade[k], bytes).map_err(|e| format!("Seite {} kann nicht geschrieben werden: {e}", k + 1))?;
            let n = fertig.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            fortschritt(n, gesamt);
            Ok(())
        })
        .collect();
    for e in ergebnisse {
        e?;
    }
    schreibe_pdf(&ordner.join("fotobuch.pdf"), &pfade, b, h)?;
    Ok(json!({"ordner": ordner.to_string_lossy(), "seiten": gesamt, "breite": b, "hoehe": h, "pdf": ordner.join("fotobuch.pdf").to_string_lossy()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foto_ueber_die_doppelseite_wird_je_haelfte_gerendert() {
        let dir = tempfile::tempdir().unwrap();
        // links rot, rechts blau: Hälften müssen zu den richtigen Seiten gehören
        let bild = RgbImage::from_fn(400, 200, |x, _| if x < 200 { Rgb([220, 20, 20]) } else { Rgb([20, 20, 220]) });
        let quelle = dir.path().join("p.jpg");
        bild.save(&quelle).unwrap();
        let links = Auftrag { nr: 1, fotos: vec![SeitenFoto { pfad: quelle.clone(), endung: "jpg".into(), feld: [0.0, 0.0, 200.0, 100.0] }] };
        let rechts = Auftrag { nr: 2, fotos: vec![SeitenFoto { pfad: quelle, endung: "jpg".into(), feld: [-100.0, 0.0, 200.0, 100.0] }] };
        let l = rendere_seite(&links, 800, 800);
        let r = rendere_seite(&rechts, 800, 800);
        let (pl, pr) = (l.get_pixel(400, 400).0, r.get_pixel(400, 400).0);
        assert!(pl[0] > 150 && pl[2] < 80, "linke Seite zeigt die linke Hälfte: {pl:?}");
        assert!(pr[2] > 150 && pr[0] < 80, "rechte Seite zeigt die rechte Hälfte: {pr:?}");
    }

    #[test]
    fn seite_wird_gerendert_und_pdf_ist_gueltig_aufgebaut() {
        let dir = tempfile::tempdir().unwrap();
        let bild = RgbImage::from_fn(400, 200, |x, _| Rgb([(x % 255) as u8, 80, 160]));
        let quelle = dir.path().join("a.jpg");
        bild.save(&quelle).unwrap();
        let auftrag = Auftrag { nr: 1, fotos: vec![SeitenFoto { pfad: quelle.clone(), endung: "jpg".into(), feld: [0.0, 0.0, 100.0, 100.0] }] };
        let ziel = dir.path().join("aus");
        let r = exportiere(&[auftrag], 1, "quadrat", &ziel, &|_, _| {}).unwrap();
        assert_eq!(r["seiten"], 2);
        let s1 = image::open(ziel.join("seite-001.jpg")).unwrap().to_rgb8();
        assert_eq!(s1.dimensions(), (2480, 2480));
        assert_ne!(s1.get_pixel(1240, 1240).0, [255, 255, 255], "Foto sichtbar");
        let s2 = image::open(ziel.join("seite-002.jpg")).unwrap().to_rgb8();
        assert!(s2.get_pixel(10, 10).0.iter().all(|&v| v > 250), "freie Seite ist weiss");
        let pdf = std::fs::read(ziel.join("fotobuch.pdf")).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Count 2"));
        assert!(text.contains("/MediaBox [0 0 595.20 595.20]"));
    }
}
