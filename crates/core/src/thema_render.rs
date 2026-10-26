//! Zeichnet die Gestaltung: Seitenhintergründe, Fotorahmen, Beschriftung und Deckblätter.
//! Schriften sind eingebunden (alle frei unter der SIL Open Font License, siehe docs/schriften.md).

use std::sync::OnceLock;

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use image::{Rgb, RgbImage};

use crate::gestaltung::{mische_farbe, Farbe, Hintergrund, Rahmen, Schrift, Stil};
use crate::seitenexport::{foto_fuellend, SeitenFoto};

// ------------------------------------------------------------------ Schriften

macro_rules! schrift {
    ($name:ident, $datei:expr) => {
        fn $name() -> &'static FontRef<'static> {
            static F: OnceLock<FontRef<'static>> = OnceLock::new();
            F.get_or_init(|| FontRef::try_from_slice(include_bytes!(concat!("../fonts/", $datei))).expect("eingebundene Schrift"))
        }
    };
}
schrift!(poppins_medium, "Poppins-Medium.ttf");
schrift!(poppins_regular, "Poppins-Regular.ttf");
schrift!(lora_kursiv, "Lora-Italic-Variable.ttf");
schrift!(barlow_fett, "BarlowCondensed-Bold.ttf");
schrift!(barlow_mittel, "BarlowCondensed-Medium.ttf");
schrift!(caveat, "Caveat-Variable.ttf");
schrift!(rubik_mono, "RubikMonoOne-Regular.ttf");
schrift!(plex_mono, "IBMPlexMono-Medium.ttf");

/// (Titelschrift, Textschrift) eines Themas
fn schriften(s: Schrift) -> (&'static FontRef<'static>, &'static FontRef<'static>) {
    match s {
        Schrift::Poppins => (poppins_medium(), poppins_regular()),
        Schrift::Lora => (lora_kursiv(), lora_kursiv()),
        Schrift::Barlow => (barlow_fett(), barlow_mittel()),
        Schrift::Caveat => (rubik_mono(), caveat()),
        Schrift::Plex => (plex_mono(), plex_mono()),
    }
}

pub fn text_breite(font: &FontRef, px: f32, s: &str, abstand: f32) -> f32 {
    let sk = font.as_scaled(PxScale::from(px));
    s.chars().map(|c| sk.h_advance(sk.glyph_id(c)) + abstand).sum::<f32>() - if s.is_empty() { 0.0 } else { abstand }
}

fn mische(dst: &mut Rgb<u8>, c: Farbe, a: f32) {
    let a = a.clamp(0.0, 1.0);
    for i in 0..3 {
        dst.0[i] = (dst.0[i] as f32 * (1.0 - a) + c[i] as f32 * a).round() as u8;
    }
}

/// Text mit Grundlinie bei `y` ab `x`. `abstand` ist der zusätzliche Buchstabenabstand in Pixeln.
pub fn zeichne_text(img: &mut RgbImage, font: &FontRef, px: f32, farbe: Farbe, x: f32, y: f32, s: &str, abstand: f32) {
    let sk = font.as_scaled(PxScale::from(px));
    let mut cx = x;
    for c in s.chars() {
        let id = sk.glyph_id(c);
        let g = id.with_scale_and_position(PxScale::from(px), point(cx, y));
        if let Some(og) = font.outline_glyph(g) {
            let b = og.px_bounds();
            let (w, h) = (img.width() as i32, img.height() as i32);
            og.draw(|gx, gy, cov| {
                let (ix, iy) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
                if ix >= 0 && iy >= 0 && ix < w && iy < h && cov > 0.0 {
                    mische(img.get_pixel_mut(ix as u32, iy as u32), farbe, cov);
                }
            });
        }
        cx += sk.h_advance(id) + abstand;
    }
}

/// Verkleinert die Schrift, bis der Text in `max` Pixel passt (nicht unter 45 %).
fn passend(font: &FontRef, px: f32, s: &str, abstand: f32, max: f32) -> f32 {
    let w = text_breite(font, px, s, abstand);
    if w <= max || w <= 0.0 {
        px
    } else {
        (px * max / w).max(px * 0.45)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ausr {
    Links,
    Mitte,
    Rechts,
}

#[allow(clippy::too_many_arguments)]
fn text_ausgerichtet(img: &mut RgbImage, font: &FontRef, px: f32, farbe: Farbe, x: f32, y: f32, s: &str, abstand: f32, ausr: Ausr, max: f32) {
    let px = passend(font, px, s, abstand, max);
    let w = text_breite(font, px, s, abstand);
    let x0 = match ausr {
        Ausr::Links => x,
        Ausr::Mitte => x - w / 2.0,
        Ausr::Rechts => x - w,
    };
    zeichne_text(img, font, px, farbe, x0, y, s, abstand);
}

// ------------------------------------------------------------------ Hintergründe

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263)).wrapping_add(seed.wrapping_mul(2246822519));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

fn kraft_pixel(x: u32, y: u32, basis: Farbe) -> Rgb<u8> {
    let mut d: i32 = (hash(x, y, 1) % 7) as i32 - 3;
    d += (hash(x / 24, y / 24, 2) % 9) as i32 - 4;
    if (x as f32 * 0.30 + y as f32 * 0.95).rem_euclid(9.0) < 1.5 {
        d -= 7;
    }
    if hash(x, y, 3) % 131 == 0 {
        d -= 16;
    }
    Rgb([0, 1, 2].map(|i| (basis[i] as i32 + d).clamp(0, 255) as u8))
}

fn leinen_pixel(x: u32, y: u32, basis: Farbe) -> Rgb<u8> {
    let mut d: i32 = (hash(x, y, 4) % 9) as i32 - 4;
    if y % 6 == 0 {
        d += 9;
    }
    if x % 6 == 0 {
        d -= 8;
    }
    if (x / 6 + y / 6) % 11 == 0 {
        d -= 5;
    }
    Rgb([0, 1, 2].map(|i| (basis[i] as i32 + d).clamp(0, 255) as u8))
}

fn wellen(img: &mut RgbImage, farbe: Farbe, start: f32, hoehe_anteil: f32, ruhe: f32, alphas: [f32; 2]) {
    let (w, h) = (img.width(), img.height());
    for x in 0..w {
        let t = x as f32 / w as f32 * std::f32::consts::TAU * 3.5;
        let kanten = [start * h as f32 + t.sin() * hoehe_anteil * h as f32, (start + ruhe) * h as f32 + (t + 1.2).sin() * hoehe_anteil * h as f32];
        for (k, &y0) in kanten.iter().enumerate() {
            for y in (y0.max(0.0) as u32)..h {
                mische(img.get_pixel_mut(x, y), farbe, alphas[k]);
            }
        }
    }
}

/// Die Seite ohne Fotos: Grundfarbe, Papierstruktur oder Welle.
pub fn seite_hintergrund(b: u32, h: u32, stil: &Stil) -> RgbImage {
    let mut img = match stil.hintergrund {
        Hintergrund::Kraft => RgbImage::from_fn(b, h, |x, y| kraft_pixel(x, y, stil.seite)),
        _ => RgbImage::from_pixel(b, h, Rgb(stil.seite)),
    };
    if stil.hintergrund == Hintergrund::Meer {
        wellen(&mut img, stil.akzent, 0.885, 0.010, 0.04, [0.30, 0.55]);
    }
    img
}

// ------------------------------------------------------------------ Fotorahmen

fn rechteck(img: &mut RgbImage, x: i64, y: i64, w: i64, h: i64, farbe: Farbe, alpha: f32) {
    let (iw, ih) = (img.width() as i64, img.height() as i64);
    for yy in y.max(0)..(y + h).min(ih) {
        for xx in x.max(0)..(x + w).min(iw) {
            mische(img.get_pixel_mut(xx as u32, yy as u32), farbe, alpha);
        }
    }
}

fn einsetzen(seite: &mut RgbImage, bild: &RgbImage, x: i64, y: i64) {
    image::imageops::replace(seite, bild, x, y);
}

fn bilinear(img: &RgbImage, u: f32, v: f32) -> [f32; 3] {
    let (x0, y0) = (u.floor() as u32, v.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(img.width() - 1), (y0 + 1).min(img.height() - 1));
    let (fx, fy) = (u - x0 as f32, v - y0 as f32);
    let mut out = [0.0; 3];
    for i in 0..3 {
        let a = img.get_pixel(x0, y0).0[i] as f32 * (1.0 - fx) + img.get_pixel(x1, y0).0[i] as f32 * fx;
        let b = img.get_pixel(x0, y1).0[i] as f32 * (1.0 - fx) + img.get_pixel(x1, y1).0[i] as f32 * fx;
        out[i] = a * (1.0 - fy) + b * fy;
    }
    out
}

/// Legt eine Karte gedreht (Grad) mit Schatten auf die Seite. Mittelpunkt (cx, cy).
fn karte_gedreht(seite: &mut RgbImage, karte: &RgbImage, cx: f32, cy: f32, grad: f32, schatten: f32) {
    let (kw, kh) = (karte.width() as f32, karte.height() as f32);
    let (s, c) = grad.to_radians().sin_cos();
    let r = (kw * kw + kh * kh).sqrt() / 2.0 + schatten * 3.0 + 3.0;
    let (iw, ih) = (seite.width() as f32, seite.height() as f32);
    let (x0, x1) = ((cx - r).max(0.0) as u32, ((cx + r).min(iw - 1.0)) as u32);
    let (y0, y1) = ((cy - r).max(0.0) as u32, ((cy + r).min(ih - 1.0)) as u32);
    let zurueck = |dx: f32, dy: f32| (dx * c + dy * s + kw / 2.0, -dx * s + dy * c + kh / 2.0);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let (u, v) = zurueck(dx, dy);
            if u >= 0.0 && v >= 0.0 && u < kw - 1.0 && v < kh - 1.0 {
                let p = bilinear(karte, u, v);
                *seite.get_pixel_mut(x, y) = Rgb([p[0].round() as u8, p[1].round() as u8, p[2].round() as u8]);
            } else {
                let (u2, v2) = zurueck(dx - schatten * 0.6, dy - schatten * 1.1);
                let aussen = ((u2 - kw / 2.0).abs() - kw / 2.0).max((v2 - kh / 2.0).abs() - kh / 2.0).max(0.0);
                if aussen < schatten * 1.5 {
                    mische(seite.get_pixel_mut(x, y), [30, 20, 10], 0.30 * (1.0 - aussen / (schatten * 1.5)));
                }
            }
        }
    }
}

/// Setzt ein Foto in sein Feld (Pixel) mit dem Rahmen des Stils. `voll`: randlos, ohne Rahmen. `zufall`: für die leichte Drehung.
#[allow(clippy::too_many_arguments)]
pub fn foto_platzieren(seite: &mut RgbImage, foto: &SeitenFoto, x: i64, y: i64, w: u32, h: u32, stil: &Stil, voll: bool, zufall: u32) {
    let rahmen = if voll { Rahmen::Keiner } else { stil.rahmen };
    let kante = seite.width().min(seite.height()) as f32;
    match rahmen {
        Rahmen::Keiner => einsetzen(seite, &foto_fuellend(foto, w, h), x, y),
        Rahmen::Weiss => {
            let rand = (kante * 0.0125).round().max(2.0) as i64;
            for i in 1..=8i64 {
                rechteck(seite, x - i, y - i + rand / 2, w as i64 + 2 * i, h as i64 + 2 * i, [0, 0, 0], 0.035);
            }
            rechteck(seite, x, y, w as i64, h as i64, [255, 255, 255], 1.0);
            let (iw, ih) = ((w as i64 - 2 * rand).max(1) as u32, (h as i64 - 2 * rand).max(1) as u32);
            einsetzen(seite, &foto_fuellend(foto, iw, ih), x + rand, y + rand);
        }
        Rahmen::Linie => {
            let lw = (kante / 900.0).round().max(2.0) as i64;
            rechteck(seite, x, y, w as i64, h as i64, stil.text, 1.0);
            let (iw, ih) = ((w as i64 - 2 * lw).max(1) as u32, (h as i64 - 2 * lw).max(1) as u32);
            einsetzen(seite, &foto_fuellend(foto, iw, ih), x + lw, y + lw);
        }
        Rahmen::Polaroid => {
            // Karte etwas kleiner als das Feld, damit die Drehung im Feld bleibt
            let (kw, kh) = (((w as f32) * 0.94).round() as u32, ((h as f32) * 0.94).round() as u32);
            let seitenrand = (kw.min(kh) as f32 * 0.04).round().max(2.0) as u32;
            let unten = (kh as f32 * 0.11).round().max(seitenrand as f32 * 2.0) as u32;
            let mut karte = RgbImage::from_pixel(kw, kh, Rgb([255, 255, 255]));
            let (iw, ih) = (kw.saturating_sub(2 * seitenrand).max(1), kh.saturating_sub(seitenrand + unten).max(1));
            einsetzen(&mut karte, &foto_fuellend(foto, iw, ih), seitenrand as i64, seitenrand as i64);
            let grad = (zufall % 41) as f32 / 10.0 - 2.0; // −2,0 … +2,0 Grad
            let grad = if grad.abs() < 0.6 { 1.2 } else { grad };
            karte_gedreht(seite, &karte, x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0, grad, kante * 0.006);
        }
    }
}

// ------------------------------------------------------------------ Beschriftung

/// Unterschrift und Seitenzahl am unteren Rand (nicht auf randlosen Seiten).
pub fn beschriftung(seite: &mut RgbImage, stil: &Stil, text: Option<&str>, nummer: usize) {
    let (b, h) = (seite.width() as f32, seite.height() as f32);
    let (_, klein) = schriften(stil.thema.schrift);
    let handschrift = stil.thema.schrift == Schrift::Caveat;
    let mut px = b.min(h) * if handschrift { 0.026 } else { 0.0175 };
    if stil.thema.id == "nacht" {
        px *= 1.15;
    }
    let abstand = if stil.thema.gross { px * 0.12 } else { 0.0 };
    let rand = 0.05 * b;
    let unten = h - rand * 0.42;
    let links = ((stil.bereich[0] + 0.05 * stil.bereich[2]) / 100.0) as f32 * b;
    let rechts = b - links;
    let meer = stil.hintergrund == Hintergrund::Meer;
    let gross = |s: &str| if stil.thema.gross { s.to_uppercase() } else { s.to_string() };

    let mut zeile = text.map(gross);
    if stil.thema.id == "galerie" {
        let nr = format!("NR. {nummer}");
        zeile = Some(match zeile {
            Some(t) => format!("{nr} · {t}"),
            None => nr,
        });
    }
    if let Some(t) = zeile.filter(|_| stil.beschriftung) {
        let y = if meer { h * 0.865 } else { unten };
        let farbe = if stil.thema.id == "meer" || meer { stil.akzent } else { stil.text };
        text_ausgerichtet(seite, klein, px, farbe, links, y, &t, abstand, Ausr::Links, (rechts - links) * 0.78);
    }
    if stil.seitenzahl && stil.thema.id != "galerie" {
        let (s, farbe) = match stil.thema.id {
            "leinen" => (format!("· {nummer} ·"), mische_farbe(stil.text, stil.seite, 0.35)),
            "nacht" => (format!("{nummer:02}"), stil.akzent),
            _ if meer => (nummer.to_string(), [234, 243, 246]),
            _ => (nummer.to_string(), mische_farbe(stil.text, stil.seite, 0.4)),
        };
        if stil.thema.id == "leinen" {
            text_ausgerichtet(seite, klein, px, farbe, b / 2.0, unten, &s, 0.0, Ausr::Mitte, b);
        } else {
            text_ausgerichtet(seite, klein, px, farbe, rechts, unten, &s, abstand, Ausr::Rechts, b);
        }
    }
}

// ------------------------------------------------------------------ Deckblatt

fn verlauf(a: Farbe, b: Farbe, t: f32) -> Farbe {
    mische_farbe(a, b, t)
}

fn foto_in(img: &mut RgbImage, foto: Option<&SeitenFoto>, x: f32, y: f32, w: f32, h: f32) {
    let (bw, bh) = (img.width() as f32, img.height() as f32);
    let (px, py, pw, ph) = ((x / 100.0 * bw).round() as i64, (y / 100.0 * bh).round() as i64, (w / 100.0 * bw).round().max(1.0) as u32, (h / 100.0 * bh).round().max(1.0) as u32);
    match foto {
        Some(f) => einsetzen(img, &foto_fuellend(f, pw, ph), px, py),
        None => rechteck(img, px, py, pw as i64, ph as i64, [205, 208, 214], 1.0),
    }
}

/// Das Deckblatt: ein eigenes Bild in der Grösse einer Seite.
pub fn rendere_deckblatt(stil: &Stil, b: u32, h: u32, titel: &str, untertitel: &str, name: &str, foto: Option<&SeitenFoto>) -> RgbImage {
    use crate::gestaltung::Deckblatt as D;
    let (bf, hf) = (b as f32, h as f32);
    let (gross, klein) = schriften(stil.thema.schrift);
    let prozent_x = |p: f32| p / 100.0 * bf;
    let prozent_y = |p: f32| p / 100.0 * hf;
    let weich = mische_farbe(stil.text, stil.seite, 0.35);
    let mut img;
    match stil.thema.deckblatt {
        D::Klar => {
            img = RgbImage::from_pixel(b, h, Rgb(stil.seite));
            foto_in(&mut img, foto, 8.0, 8.0, 84.0, 60.0);
            rechteck(&mut img, prozent_x(8.0) as i64, prozent_y(73.0) as i64, prozent_x(7.0) as i64, (hf * 0.008).max(2.0) as i64, stil.akzent, 1.0);
            text_ausgerichtet(&mut img, gross, hf * 0.075, stil.text, prozent_x(8.0), prozent_y(84.0), titel, 0.0, Ausr::Links, bf * 0.84);
            text_ausgerichtet(&mut img, klein, hf * 0.032, stil.akzent, prozent_x(8.0), prozent_y(90.5), untertitel, 0.0, Ausr::Links, bf * 0.55);
            text_ausgerichtet(&mut img, klein, hf * 0.028, weich, prozent_x(92.0), prozent_y(90.5), name, 0.0, Ausr::Rechts, bf * 0.3);
        }
        D::Leinen => {
            img = RgbImage::from_fn(b, h, |x, y| leinen_pixel(x, y, stil.akzent));
            let rand = (hf * 0.016).round() as i64;
            let (fx, fy, fw, fh) = (prozent_x(30.0).round() as i64, prozent_y(13.0).round() as i64, prozent_x(40.0).round() as u32, prozent_y(44.0).round() as u32);
            rechteck(&mut img, fx - rand, fy - rand, fw as i64 + 2 * rand, fh as i64 + 2 * rand, [251, 250, 246], 1.0);
            foto_in(&mut img, foto, 30.0, 13.0, 40.0, 44.0);
            let hell = [244, 246, 241];
            text_ausgerichtet(&mut img, gross, hf * 0.085, hell, bf / 2.0, prozent_y(71.0), titel, 0.0, Ausr::Mitte, bf * 0.84);
            text_ausgerichtet(&mut img, klein, hf * 0.026, hell, bf / 2.0, prozent_y(79.0), &untertitel.to_uppercase(), hf * 0.006, Ausr::Mitte, bf * 0.84);
            text_ausgerichtet(&mut img, klein, hf * 0.03, mische_farbe(hell, stil.akzent, 0.3), bf / 2.0, prozent_y(86.0), name, 0.0, Ausr::Mitte, bf * 0.7);
        }
        D::Nacht => {
            img = RgbImage::from_pixel(b, h, Rgb(stil.seite));
            foto_in(&mut img, foto, 0.0, 0.0, 100.0, 100.0);
            let start = (hf * 0.38) as u32;
            for y in start..h {
                let a = ((y - start) as f32 / (hf - start as f32)).powf(0.9) * 0.93;
                for x in 0..b {
                    mische(img.get_pixel_mut(x, y), stil.seite, a);
                }
            }
            text_ausgerichtet(&mut img, gross, hf * 0.125, stil.text, prozent_x(7.0), prozent_y(82.0), &titel.to_uppercase(), hf * 0.002, Ausr::Links, bf * 0.86);
            text_ausgerichtet(&mut img, klein, hf * 0.04, stil.akzent, prozent_x(7.0), prozent_y(89.0), &untertitel.to_uppercase(), hf * 0.012, Ausr::Links, bf * 0.86);
            text_ausgerichtet(&mut img, klein, hf * 0.034, mische_farbe(stil.text, stil.seite, 0.3), prozent_x(7.0), prozent_y(94.5), &name.to_uppercase(), hf * 0.006, Ausr::Links, bf * 0.86);
        }
        D::Kraft => {
            img = RgbImage::from_fn(b, h, |x, y| kraft_pixel(x, y, stil.seite));
            let (kw, kh) = (prozent_x(40.0).round() as u32, prozent_y(50.0).round() as u32);
            let rand = (kw.min(kh) as f32 * 0.04).round().max(2.0) as u32;
            let unten = (kh as f32 * 0.11).round() as u32;
            let mut karte = RgbImage::from_pixel(kw, kh, Rgb([255, 255, 255]));
            let (iw, ih) = (kw - 2 * rand, kh - rand - unten);
            match foto {
                Some(f) => einsetzen(&mut karte, &foto_fuellend(f, iw, ih), rand as i64, rand as i64),
                None => rechteck(&mut karte, rand as i64, rand as i64, iw as i64, ih as i64, [205, 208, 214], 1.0),
            }
            karte_gedreht(&mut img, &karte, prozent_x(50.0), prozent_y(10.0) + kh as f32 / 2.0, -3.0, hf * 0.008);
            let px = passend(gross, hf * 0.055, &titel.to_uppercase(), hf * 0.002, bf * 0.78);
            let tb = text_breite(gross, px, &titel.to_uppercase(), hf * 0.002);
            let (ty, pad) = (prozent_y(79.0), px * 0.45);
            let dick = (hf * 0.005).round().max(2.0) as i64;
            let (rx, ry, rw, rh) = ((bf / 2.0 - tb / 2.0 - pad) as i64, (ty - px - pad * 0.6) as i64, (tb + 2.0 * pad) as i64, (px + pad * 1.5) as i64);
            rechteck(&mut img, rx, ry, rw, dick, stil.text, 1.0);
            rechteck(&mut img, rx, ry + rh - dick, rw, dick, stil.text, 1.0);
            rechteck(&mut img, rx, ry, dick, rh, stil.text, 1.0);
            rechteck(&mut img, rx + rw - dick, ry, dick, rh, stil.text, 1.0);
            zeichne_text(&mut img, gross, px, stil.text, bf / 2.0 - tb / 2.0, ty, &titel.to_uppercase(), hf * 0.002);
            text_ausgerichtet(&mut img, klein, hf * 0.06, stil.text, bf / 2.0, prozent_y(90.0), untertitel, 0.0, Ausr::Mitte, bf * 0.84);
            text_ausgerichtet(&mut img, klein, hf * 0.045, stil.akzent, bf / 2.0, prozent_y(96.0), name, 0.0, Ausr::Mitte, bf * 0.7);
        }
        D::Meer => {
            let (oben, mitte, unten) = (verlauf(stil.akzent, [255, 255, 255], 0.65), verlauf(stil.akzent, [255, 255, 255], 0.28), stil.akzent);
            img = RgbImage::from_fn(b, h, |_, y| {
                let t = y as f32 / hf;
                Rgb(if t < 0.55 { verlauf(oben, mitte, t / 0.55) } else { verlauf(mitte, unten, (t - 0.55) / 0.45) })
            });
            let rand = (hf * 0.014).round() as i64;
            let (fx, fy, fw, fh) = (prozent_x(11.0).round() as i64, prozent_y(12.0).round() as i64, prozent_x(78.0).round() as u32, prozent_y(50.0).round() as u32);
            rechteck(&mut img, fx - rand, fy - rand, fw as i64 + 2 * rand, fh as i64 + 2 * rand, [255, 255, 255], 1.0);
            foto_in(&mut img, foto, 11.0, 12.0, 78.0, 50.0);
            wellen(&mut img, [255, 255, 255], 0.86, 0.012, 0.045, [0.18, 0.18]);
            let weiss = [255, 255, 255];
            text_ausgerichtet(&mut img, gross, hf * 0.08, weiss, prozent_x(11.0), prozent_y(79.0), titel, 0.0, Ausr::Links, bf * 0.78);
            text_ausgerichtet(&mut img, klein, hf * 0.033, weiss, prozent_x(11.0), prozent_y(85.5), untertitel, 0.0, Ausr::Links, bf * 0.78);
            text_ausgerichtet(&mut img, klein, hf * 0.03, mische_farbe(weiss, stil.akzent, 0.2), prozent_x(11.0), prozent_y(91.0), name, 0.0, Ausr::Links, bf * 0.78);
        }
        D::Galerie => {
            img = RgbImage::from_pixel(b, h, Rgb(stil.seite));
            let lw = (hf / 700.0).round().max(2.0) as i64;
            let (fx, fy, fw, fh) = (prozent_x(33.0).round() as i64, prozent_y(14.0).round() as i64, prozent_x(34.0).round() as u32, prozent_y(45.0).round() as u32);
            rechteck(&mut img, fx - lw, fy - lw, fw as i64 + 2 * lw, fh as i64 + 2 * lw, stil.text, 1.0);
            foto_in(&mut img, foto, 33.0, 14.0, 34.0, 45.0);
            let x = prozent_x(33.0);
            text_ausgerichtet(&mut img, gross, hf * 0.036, stil.text, x, prozent_y(69.0), &titel.to_uppercase(), hf * 0.004, Ausr::Links, bf * 0.5);
            text_ausgerichtet(&mut img, klein, hf * 0.026, stil.text, x, prozent_y(74.0), &untertitel.to_uppercase(), hf * 0.002, Ausr::Links, bf * 0.5);
            if !name.is_empty() {
                let r = hf * 0.0075;
                let (cx, cy) = (x + r, prozent_y(79.5) - r);
                for yy in (cy - r) as i64..=(cy + r) as i64 {
                    for xx in (cx - r) as i64..=(cx + r) as i64 {
                        if ((xx as f32 - cx).powi(2) + (yy as f32 - cy).powi(2)).sqrt() <= r && xx >= 0 && yy >= 0 && (xx as u32) < b && (yy as u32) < h {
                            *img.get_pixel_mut(xx as u32, yy as u32) = Rgb(stil.akzent);
                        }
                    }
                }
                text_ausgerichtet(&mut img, klein, hf * 0.026, stil.text, x + r * 3.0, prozent_y(79.5), &name.to_uppercase(), hf * 0.002, Ausr::Links, bf * 0.45);
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gestaltung::Gestaltung;

    fn stil(thema: &str) -> Stil {
        let mut g = Gestaltung::default();
        g.thema = thema.into();
        g.stil()
    }

    #[test]
    fn schriften_zeichnen_und_haben_umlaute() {
        let mut img = RgbImage::from_pixel(600, 120, Rgb([255, 255, 255]));
        for s in [Schrift::Poppins, Schrift::Lora, Schrift::Barlow, Schrift::Caveat, Schrift::Plex] {
            let (gross, klein) = schriften(s);
            for f in [gross, klein] {
                assert!(text_breite(f, 40.0, "Zürich Ärger", 0.0) > 100.0, "Umlaute haben Breite");
                zeichne_text(&mut img, f, 40.0, [0, 0, 0], 10.0, 80.0, "Zürich Ärger 14.–17. Mai", 0.0);
            }
        }
        assert!(img.pixels().any(|p| p.0 != [255, 255, 255]), "Text sichtbar");
    }

    #[test]
    fn hintergruende_haben_die_erwartete_struktur() {
        let kraft = seite_hintergrund(400, 300, &stil("kraft"));
        let grau: std::collections::HashSet<[u8; 3]> = kraft.pixels().map(|p| p.0).collect();
        assert!(grau.len() > 10, "Kraftpapier ist strukturiert");
        let meer = seite_hintergrund(400, 300, &stil("meer"));
        assert_eq!(meer.get_pixel(5, 5).0, [234, 243, 246], "oben hell");
        assert_ne!(meer.get_pixel(5, 295).0, [234, 243, 246], "unten Welle");
        let klar = seite_hintergrund(40, 30, &stil("klar"));
        assert!(klar.pixels().all(|p| p.0 == [255, 255, 255]));
    }

    #[test]
    fn deckblaetter_aller_themen_sind_gezeichnet_und_verschieden() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("t.jpg");
        RgbImage::from_fn(300, 200, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 120])).save(&q).unwrap();
        let foto = SeitenFoto { pfad: q, endung: "jpg".into(), feld: [0.0, 0.0, 100.0, 100.0] };
        let mut summen = std::collections::HashSet::new();
        for t in crate::gestaltung::THEMEN.iter() {
            let img = rendere_deckblatt(&stil(t.id), 700, 525, "New York 2026", "14.–17. Mai 2026", "Familie Muster", Some(&foto));
            assert_eq!(img.dimensions(), (700, 525));
            let s: u64 = img.pixels().map(|p| p.0.iter().map(|&v| v as u64).sum::<u64>()).sum();
            assert!(summen.insert(s), "Deckblatt {} unterscheidet sich", t.id);
        }
    }

    #[test]
    fn polaroid_dreht_und_bleibt_im_feld() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("p.jpg");
        RgbImage::from_pixel(300, 200, Rgb([200, 30, 30])).save(&q).unwrap();
        let foto = SeitenFoto { pfad: q, endung: "jpg".into(), feld: [0.0; 4] };
        let mut s = seite_hintergrund(800, 600, &stil("kraft"));
        foto_platzieren(&mut s, &foto, 200, 150, 400, 300, &stil("kraft"), false, 7);
        assert!(s.get_pixel(400, 280).0[0] > 150 && s.get_pixel(400, 280).0[1] < 90, "Foto in der Mitte");
        assert_ne!(s.get_pixel(5, 5).0, [255, 255, 255]);
    }
}
