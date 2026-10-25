//! Einlesen von Fotos: Aufnahmezeit, GPS, Grösse, Hashes, Schärfe. Rein lokal, ohne KI.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Local, NaiveDateTime};
use image::imageops::{self, FilterType};
use image::{GrayImage, RgbImage};
use sha2::{Digest, Sha256};

use crate::foto::Foto;

pub const JPEG_EXT: &[&str] = &[".jpg", ".jpeg", ".jpe"];
pub const HEIC_EXT: &[&str] = &[".heic", ".heif"];
pub const PNG_EXT: &[&str] = &[".png"];
pub const OTHER_EXT: &[&str] = &[".tif", ".tiff", ".webp", ".bmp"];
pub const RAW_EXT: &[&str] = &[".dng", ".cr2", ".cr3", ".nef", ".arw", ".orf", ".rw2", ".raf", ".pef", ".srw"];
const SCREENSHOT_HINTS: &[&str] = &["screenshot", "bildschirmfoto", "bildschirmaufnahme", "screen shot", "capture"];
pub const EIGENE_ORDNER: &[&str] = &[".fotobuch", "fotobuch-bericht", "fotobuch-export"];
/// Längste Kante der Analysebilder in Pixeln
pub const ANALYSE_KANTE: u32 = 512;

fn endung_von(p: &Path) -> String {
    p.extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

fn ist_unterstuetzt(endung: &str) -> bool {
    [JPEG_EXT, HEIC_EXT, PNG_EXT, OTHER_EXT, RAW_EXT].iter().any(|l| l.contains(&endung))
}

/// Alle unterstützten Bilddateien unterhalb von `ordner`, sortiert nach Pfad (ohne Gross-/Kleinschreibung).
pub fn finde_fotos(ordner: &Path) -> Vec<PathBuf> {
    let mut gefunden = Vec::new();
    let mut stapel = vec![ordner.to_path_buf()];
    while let Some(dir) = stapel.pop() {
        let Ok(eintraege) = std::fs::read_dir(&dir) else { continue };
        for e in eintraege.flatten() {
            let pfad = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let Ok(typ) = e.file_type() else { continue };
            if typ.is_dir() {
                if EIGENE_ORDNER.contains(&name.to_lowercase().as_str()) {
                    continue;
                }
                stapel.push(pfad);
            } else if typ.is_file() && !name.starts_with("._") && ist_unterstuetzt(&endung_von(&pfad)) {
                gefunden.push(pfad);
            }
        }
    }
    gefunden.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    gefunden
}

pub fn entfernung_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = 6371.0;
    let (p1, p2) = (a.0.to_radians(), b.0.to_radians());
    let dp = p2 - p1;
    let dl = (b.1 - a.1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

// ---------------------------------------------------------------------------- EXIF

#[derive(Default)]
struct ExifDaten {
    zeit: Option<NaiveDateTime>,
    gps: Option<(f64, f64)>,
    marke: String,
    modell: String,
    orientierung: u32,
}

fn ascii(feld: &exif::Field) -> String {
    match &feld.value {
        exif::Value::Ascii(teile) => teile
            .iter()
            .map(|t| String::from_utf8_lossy(t).trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string(),
        _ => String::new(),
    }
}

fn parse_zeit(text: &str) -> Option<NaiveDateTime> {
    let t: String = text.trim().chars().take(19).collect();
    for fmt in ["%Y:%m:%d %H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(z) = NaiveDateTime::parse_from_str(&t, fmt) {
            if chrono::Datelike::year(&z) >= 1990 {
                return Some(z);
            }
        }
    }
    None
}

fn dms(feld: &exif::Field, ref_: &str) -> Option<f64> {
    if let exif::Value::Rational(teile) = &feld.value {
        if teile.len() < 3 {
            return None;
        }
        let grad = teile[0].to_f64() + teile[1].to_f64() / 60.0 + teile[2].to_f64() / 3600.0;
        Some(if ref_ == "S" || ref_ == "W" { -grad } else { grad })
    } else {
        None
    }
}

fn lies_exif(exif: &exif::Exif) -> ExifDaten {
    let mut d = ExifDaten { orientierung: 1, ..Default::default() };
    for tag in [exif::Tag::DateTimeOriginal, exif::Tag::DateTimeDigitized, exif::Tag::DateTime] {
        if let Some(f) = exif.get_field(tag, exif::In::PRIMARY) {
            if let Some(z) = parse_zeit(&ascii(f)) {
                d.zeit = Some(z);
                break;
            }
        }
    }
    let ref_lat = exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY).map(ascii).unwrap_or_else(|| "N".into());
    let ref_lon = exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY).map(ascii).unwrap_or_else(|| "E".into());
    if let (Some(la), Some(lo)) = (
        exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY),
        exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY),
    ) {
        if let (Some(lat), Some(lon)) = (dms(la, &ref_lat), dms(lo, &ref_lon)) {
            if !(lat == 0.0 && lon == 0.0) && lat.is_finite() && lon.is_finite() {
                d.gps = Some((lat, lon));
            }
        }
    }
    d.marke = exif.get_field(exif::Tag::Make, exif::In::PRIMARY).map(ascii).unwrap_or_default();
    d.modell = exif.get_field(exif::Tag::Model, exif::In::PRIMARY).map(ascii).unwrap_or_default();
    if let Some(f) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
        if let Some(v) = f.value.get_uint(0) {
            d.orientierung = v;
        }
    }
    d
}

// ---------------------------------------------------------------------------- Bilder laden

/// Geladenes Bild für die Analyse oder Vorschau. Die EXIF-Drehung ist noch nicht angewendet,
/// ausser bei HEIC (dort ist die Drehung Teil des Containers und schon enthalten).
pub struct Geladen {
    pub rgb: RgbImage,
    /// Echte Grösse des Originals (vor jeder Verkleinerung), bei HEIC schon gedreht.
    pub orig: (u32, u32),
    pub exif: Option<ExifRoh>,
    pub schon_gedreht: bool,
    pub aus_raw_vorschau: bool,
}

pub struct ExifRoh(exif::Exif);

fn jpeg_geschaetzt(daten: &[u8], mindest_kante: u32) -> Result<(RgbImage, (u32, u32)), String> {
    let mut dec = jpeg_decoder::Decoder::new(Cursor::new(daten));
    dec.read_info().map_err(|e| e.to_string())?;
    let info = dec.info().ok_or("keine JPEG-Infos")?;
    let orig = (info.width as u32, info.height as u32);
    if info.coding_process != jpeg_decoder::CodingProcess::DctSequential
        && info.coding_process != jpeg_decoder::CodingProcess::DctProgressive
    {
        return Err("JPEG-Variante nicht unterstützt".into());
    }
    // Skalierung nur so weit, dass beide Kanten mindestens `mindest_kante` behalten
    let wunsch_b = ((orig.0 as f64) * mindest_kante as f64 / orig.0.max(orig.1) as f64).ceil().max(1.0) as u16;
    let wunsch_h = ((orig.1 as f64) * mindest_kante as f64 / orig.0.max(orig.1) as f64).ceil().max(1.0) as u16;
    let (sb, sh) = dec.scale(wunsch_b, wunsch_h).map_err(|e| e.to_string())?;
    let px = dec.decode().map_err(|e| e.to_string())?;
    let fmt = dec.info().ok_or("keine JPEG-Infos")?.pixel_format;
    let (w, h) = (sb as u32, sh as u32);
    let rgb: Vec<u8> = match fmt {
        jpeg_decoder::PixelFormat::RGB24 => px,
        jpeg_decoder::PixelFormat::L8 => px.iter().flat_map(|&g| [g, g, g]).collect(),
        _ => return Err("Farbformat nicht unterstützt".into()),
    };
    let img = RgbImage::from_raw(w, h, rgb).ok_or("Bildgrösse passt nicht")?;
    Ok((img, orig))
}

/// Grösstes eingebettetes JPEG einer RAW-Datei (Vorschaubild der Kamera).
pub fn raw_vorschau(daten: &[u8]) -> Option<&[u8]> {
    let mut beste: Option<(u64, usize)> = None;
    let mut i = 0usize;
    while i + 3 < daten.len() {
        if daten[i] == 0xFF && daten[i + 1] == 0xD8 && daten[i + 2] == 0xFF {
            let mut dec = jpeg_decoder::Decoder::new(Cursor::new(&daten[i..]));
            if dec.read_info().is_ok() {
                if let Some(info) = dec.info() {
                    let pixel = info.width as u64 * info.height as u64;
                    if pixel >= 160 * 120 && beste.map_or(true, |(p, _)| pixel > p) {
                        beste = Some((pixel, i));
                    }
                }
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    beste.map(|(_, start)| &daten[start..])
}

/// Lädt ein Bild so weit verkleinert, dass die längere Kante mindestens `mindest_kante` hat (wenn möglich).
pub fn lade_bild(pfad: &Path, daten: &[u8], endung: &str, mindest_kante: u32) -> Result<Geladen, String> {
    let exif_aus = |bytes: &[u8]| -> Option<ExifRoh> {
        exif::Reader::new().read_from_container(&mut Cursor::new(bytes)).ok().map(ExifRoh)
    };
    if HEIC_EXT.contains(&endung) {
        let cfg = heic::DecoderConfig::new();
        let out = cfg.decode(daten, heic::PixelLayout::Rgb8).map_err(|e| format!("HEIC: {e:?}"))?;
        let roh = cfg.extract_exif(daten).ok().flatten();
        let exif = roh.and_then(|r| {
            let b: &[u8] = &r;
            exif::Reader::new().read_raw(b.to_vec()).ok().map(ExifRoh)
        });
        let rgb = RgbImage::from_raw(out.width, out.height, out.data.to_vec()).ok_or("HEIC: Bildgrösse passt nicht")?;
        return Ok(Geladen { orig: (out.width, out.height), rgb, exif, schon_gedreht: true, aus_raw_vorschau: false });
    }
    if RAW_EXT.contains(&endung) {
        let exif = exif_aus(daten);
        let jpeg = raw_vorschau(daten).ok_or("RAW: keine eingebettete Vorschau gefunden")?;
        let (rgb, orig) = jpeg_geschaetzt(jpeg, mindest_kante).map_err(|e| format!("RAW-Vorschau: {e}"))?;
        // Orientierung der RAW-Datei gilt auch für die Vorschau; Grösse des Originals ist unbekannt,
        // deshalb gilt die Grösse der Vorschau.
        return Ok(Geladen { rgb, orig, exif, schon_gedreht: false, aus_raw_vorschau: true });
    }
    let exif = exif_aus(daten);
    if JPEG_EXT.contains(&endung) {
        if let Ok((rgb, orig)) = jpeg_geschaetzt(daten, mindest_kante) {
            return Ok(Geladen { rgb, orig, exif, schon_gedreht: false, aus_raw_vorschau: false });
        }
    }
    let _ = pfad;
    let img = image::load_from_memory(daten).map_err(|e| e.to_string())?;
    let orig = (img.width(), img.height());
    Ok(Geladen { rgb: img.to_rgb8(), orig, exif, schon_gedreht: false, aus_raw_vorschau: false })
}

/// Wendet die EXIF-Orientierung (1 bis 8) auf ein Bild an.
pub fn drehe(img: RgbImage, orientierung: u32) -> RgbImage {
    match orientierung {
        2 => imageops::flip_horizontal(&img),
        3 => imageops::rotate180(&img),
        4 => imageops::flip_vertical(&img),
        5 => imageops::flip_horizontal(&imageops::rotate90(&img)),
        6 => imageops::rotate90(&img),
        7 => imageops::flip_horizontal(&imageops::rotate270(&img)),
        8 => imageops::rotate270(&img),
        _ => img,
    }
}

/// Orientierung aus den EXIF-Daten, 1 wenn unbekannt oder schon angewendet.
pub fn orientierung_von(g: &Geladen) -> u32 {
    if g.schon_gedreht {
        return 1;
    }
    g.exif.as_ref().map(|e| lies_exif(&e.0).orientierung).filter(|o| (1..=8).contains(o)).unwrap_or(1)
}

fn grau_von(img: &RgbImage) -> GrayImage {
    // Gleiche Gewichte wie PIL (ITU-R 601), damit die Werte mit der Python-Version vergleichbar bleiben
    let mut g = GrayImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let v = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114 + 500) / 1000;
        g.put_pixel(x, y, image::Luma([v as u8]));
    }
    g
}

fn schaerfe(grau: &GrayImage) -> f64 {
    let (w, h) = (grau.width() as usize, grau.height() as usize);
    if w < 3 || h < 3 {
        return 0.0;
    }
    let d = grau.as_raw();
    let (mut summe, mut summe2, mut n) = (0.0f64, 0.0f64, 0.0f64);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let c = d[y * w + x] as f64;
            let l = 4.0 * c - d[(y - 1) * w + x] as f64 - d[(y + 1) * w + x] as f64 - d[y * w + x - 1] as f64 - d[y * w + x + 1] as f64;
            summe += l;
            summe2 += l * l;
            n += 1.0;
        }
    }
    let mittel = summe / n;
    (summe2 / n - mittel * mittel).max(0.0)
}

fn dhash(grau: &GrayImage) -> u64 {
    let klein = imageops::resize(grau, 9, 8, FilterType::Triangle);
    let mut wert = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            let bit = klein.get_pixel(x + 1, y)[0] > klein.get_pixel(x, y)[0];
            wert = (wert << 1) | bit as u64;
        }
    }
    wert
}

/// Verkleinert ein Bild so, dass es in `kante` x `kante` passt (kein Hochskalieren).
pub fn passe_ein(img: &RgbImage, kante: u32, filter: FilterType) -> RgbImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= kante {
        return img.clone();
    }
    let f = kante as f64 / w.max(h) as f64;
    let nw = ((w as f64 * f).round() as u32).max(1);
    let nh = ((h as f64 * f).round() as u32).max(1);
    imageops::resize(img, nw, nh, filter)
}

// ---------------------------------------------------------------------------- Analyse

fn mtime_von(pfad: &Path) -> (u64, NaiveDateTime, u128) {
    let md = std::fs::metadata(pfad);
    let (size, modified) = match md {
        Ok(m) => (m.len(), m.modified().ok()),
        Err(_) => (0, None),
    };
    let modified = modified.unwrap_or(UNIX_EPOCH);
    let ns = modified.duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let lokal: DateTime<Local> = modified.into();
    (size, lokal.naive_local(), ns)
}

/// Liest ein Foto ein. Fehler werden im Feld `fehler` festgehalten, damit ein kaputtes Bild den Lauf nicht stoppt.
pub fn analysiere_foto(abs: &Path, rel: &str) -> Foto {
    let (size, mtime, _) = mtime_von(abs);
    let endung = endung_von(abs);
    let name = abs.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut foto = Foto {
        pfad: rel.to_string(),
        name: name.clone(),
        endung: endung.clone(),
        groesse_bytes: size,
        zeit: mtime,
        zeit_quelle: "datei".into(),
        breite: 0,
        hoehe: 0,
        kamera: String::new(),
        gps: None,
        sha256: String::new(),
        dhash: 0,
        schaerfe: 0.0,
        helligkeit: 0.0,
        ist_raw: RAW_EXT.contains(&endung.as_str()),
        screenshot_verdacht: false,
        fehler: String::new(),
        hinweise: vec![],
    };
    let stamm = abs.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();

    let ergebnis = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
        let daten = std::fs::read(abs).map_err(|e| e.to_string())?;
        foto.sha256 = format!("{:x}", Sha256::digest(&daten));
        let g = lade_bild(abs, &daten, &endung, ANALYSE_KANTE)?;
        let ex = g.exif.as_ref().map(|e| lies_exif(&e.0));
        if let Some(ex) = &ex {
            if let Some(z) = ex.zeit {
                foto.zeit = z;
                foto.zeit_quelle = "exif".into();
            }
            foto.gps = ex.gps;
            let (marke, modell) = (ex.marke.trim(), ex.modell.trim());
            foto.kamera = if modell.starts_with(marke) { modell.to_string() } else { format!("{marke} {modell}") }.trim().to_string();
        }
        let orient = orientierung_von(&g);
        let (mut b, mut h) = g.orig;
        if (5..=8).contains(&orient) {
            std::mem::swap(&mut b, &mut h);
        }
        foto.breite = b;
        foto.hoehe = h;
        if g.aus_raw_vorschau {
            foto.hinweise.push("RAW: Analyse über eingebettete Vorschau".into());
        }
        let klein = passe_ein(&drehe(g.rgb, orient), ANALYSE_KANTE, FilterType::Triangle);
        let grau = grau_von(&klein);
        foto.schaerfe = schaerfe(&grau);
        foto.helligkeit = grau.as_raw().iter().map(|&v| v as f64).sum::<f64>() / grau.as_raw().len().max(1) as f64;
        foto.dhash = dhash(&grau);
        Ok(())
    }));
    match ergebnis {
        Ok(Ok(())) => {}
        Ok(Err(e)) => foto.fehler = e,
        Err(_) => foto.fehler = "Interner Fehler beim Lesen der Datei".into(),
    }

    let ist_png = PNG_EXT.contains(&endung.as_str());
    if SCREENSHOT_HINTS.iter().any(|h| stamm.contains(h)) || (ist_png && foto.kamera.is_empty()) {
        foto.screenshot_verdacht = true;
    }
    if foto.zeit_quelle == "datei" {
        foto.hinweise.push("Keine Aufnahmezeit in den Metadaten, Dateidatum benutzt".into());
    }
    foto
}

pub fn datei_zeiten(pfad: &Path) -> (u64, u128) {
    let (size, _, ns) = mtime_von(pfad);
    (size, ns)
}

