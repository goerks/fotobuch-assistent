//! Automatische Seitenlayouts.
//!
//! Die Fotos einer Seite behalten ihre Reihenfolge (Aufnahmezeit). Gesucht wird die Anordnung als
//! «Schnittbaum» (Fotos nebeneinander oder untereinander, beliebig verschachtelt), bei der die Fotos
//! zusammen die grösste Fläche belegen. Es wird nichts beschnitten, jedes Foto behält sein Seitenverhältnis.

pub const RAND: f64 = 0.05; // Anteil der Seitenbreite
pub const LUECKE: f64 = 0.02;
pub const MAX_BAUM: usize = 8;

#[derive(Clone, Debug)]
enum Baum {
    Blatt(usize),
    Teil { nebeneinander: bool, a: Box<Baum>, b: Box<Baum> },
}

fn alle_baeume(ars: &[f64], lo: usize, hi: usize) -> Vec<(f64, Baum)> {
    if hi - lo == 1 {
        return vec![(ars[lo], Baum::Blatt(lo))];
    }
    let mut aus = Vec::new();
    for k in lo + 1..hi {
        let links = alle_baeume(ars, lo, k);
        let rechts = alle_baeume(ars, k, hi);
        for (a1, t1) in &links {
            for (a2, t2) in &rechts {
                aus.push((a1 + a2, Baum::Teil { nebeneinander: true, a: Box::new(t1.clone()), b: Box::new(t2.clone()) }));
                aus.push((1.0 / (1.0 / a1 + 1.0 / a2), Baum::Teil { nebeneinander: false, a: Box::new(t1.clone()), b: Box::new(t2.clone()) }));
            }
        }
    }
    aus
}

fn seiten_verhaeltnis(b: &Baum, ars: &[f64]) -> f64 {
    match b {
        Baum::Blatt(i) => ars[*i],
        Baum::Teil { nebeneinander, a, b } => {
            let (x, y) = (seiten_verhaeltnis(a, ars), seiten_verhaeltnis(b, ars));
            if *nebeneinander { x + y } else { 1.0 / (1.0 / x + 1.0 / y) }
        }
    }
}

type Rechteck = (f64, f64, f64, f64); // x, y, Breite, Höhe

fn einpassen(ar: f64, r: Rechteck) -> Rechteck {
    let (x, y, w, h) = r;
    if w / h > ar {
        let nw = h * ar;
        (x + (w - nw) / 2.0, y, nw, h)
    } else {
        let nh = w / ar;
        (x, y + (h - nh) / 2.0, w, nh)
    }
}

fn platziere(b: &Baum, ars: &[f64], r: Rechteck, aus: &mut Vec<Rechteck>) {
    match b {
        Baum::Blatt(i) => aus[*i] = einpassen(ars[*i], r),
        Baum::Teil { nebeneinander, a, b: bb } => {
            let (a1, a2) = (seiten_verhaeltnis(a, ars), seiten_verhaeltnis(bb, ars));
            let (x, y, w, h) = r;
            if *nebeneinander {
                let w1 = (w - LUECKE) * a1 / (a1 + a2);
                let w2 = w - LUECKE - w1;
                platziere(a, ars, (x, y, w1, h), aus);
                platziere(bb, ars, (x + w1 + LUECKE, y, w2, h), aus);
            } else {
                // Höhen verhalten sich umgekehrt zu den Seitenverhältnissen
                let (i1, i2) = (1.0 / a1, 1.0 / a2);
                let h1 = (h - LUECKE) * i1 / (i1 + i2);
                let h2 = h - LUECKE - h1;
                platziere(a, ars, (x, y, w, h1), aus);
                platziere(bb, ars, (x, y + h1 + LUECKE, w, h2), aus);
            }
        }
    }
}

/// Liefert je Foto (x, y, Breite, Höhe) in Prozent der Seite.
/// `seite_ar` = Seitenbreite / Seitenhöhe.
pub fn seitenlayout(ars: &[f64], seite_ar: f64) -> Vec<[f64; 4]> {
    let n = ars.len();
    if n == 0 {
        return vec![];
    }
    let ars: Vec<f64> = ars.iter().map(|&a| if a.is_finite() && a > 0.05 { a } else { 1.5 }).collect();
    let (sw, sh) = (1.0, 1.0 / seite_ar);
    let flaeche = (sw - 2.0 * RAND, sh - 2.0 * RAND * sw);
    let mut felder = vec![(0.0, 0.0, 0.0, 0.0); n];
    let rahmen = (RAND, RAND * sw, flaeche.0, flaeche.1);
    if n <= MAX_BAUM {
        let mut bester: Option<(f64, Baum)> = None;
        for (ar, baum) in alle_baeume(&ars, 0, n) {
            let wb = flaeche.0.min(flaeche.1 * ar);
            let punkte = wb * wb / ar;
            if bester.as_ref().map_or(true, |(p, _)| punkte > *p + 1e-12) {
                bester = Some((punkte, baum));
            }
        }
        let (_, baum) = bester.unwrap();
        let ar = seiten_verhaeltnis(&baum, &ars);
        platziere(&baum, &ars, einpassen(ar, rahmen), &mut felder);
    } else {
        // Sehr viele Fotos: zwei Hälften untereinander
        let m = n / 2;
        let (oben, unten) = (seitenlayout(&ars[..m], seite_ar * 2.0), seitenlayout(&ars[m..], seite_ar * 2.0));
        return oben
            .into_iter()
            .map(|r| [r[0], r[1] / 2.0, r[2], r[3] / 2.0])
            .chain(unten.into_iter().map(|r| [r[0], 50.0 + r[1] / 2.0, r[2], r[3] / 2.0]))
            .collect();
    }
    felder.into_iter().map(|(x, y, w, h)| [x / sw * 100.0, y / sh * 100.0, w / sw * 100.0, h / sh * 100.0]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flaeche(r: &[[f64; 4]]) -> f64 {
        r.iter().map(|x| x[2] * x[3]).sum()
    }

    fn innerhalb(r: &[[f64; 4]]) -> bool {
        r.iter().all(|x| x[0] >= 0.0 && x[1] >= 0.0 && x[0] + x[2] <= 100.0001 && x[1] + x[3] <= 100.0001)
    }

    #[test]
    fn ein_foto_fuellt_die_seite_ohne_verzerrung() {
        let r = seitenlayout(&[1.5], 4.0 / 3.0);
        assert!(innerhalb(&r));
        assert!(r[0][2] > 85.0);
    }

    #[test]
    fn hochformat_neben_zwei_querformaten() {
        // Ein Hochformat und zwei Querformate: das Hochformat steht links, die Querformate untereinander
        let r = seitenlayout(&[0.75, 1.5, 1.5], 4.0 / 3.0);
        assert!(innerhalb(&r));
        assert!(r[0][0] < r[1][0], "Hochformat links");
        assert!(r[1][1] < r[2][1], "Querformate untereinander");
        assert!(flaeche(&r) > 3500.0, "gute Ausnutzung: {}", flaeche(&r));
    }

    #[test]
    fn keine_ueberlappung_und_alle_gleich_gross_genug() {
        for n in 1..=9 {
            let ars: Vec<f64> = (0..n).map(|i| if i % 3 == 0 { 0.67 } else { 1.5 }).collect();
            let r = seitenlayout(&ars, 1.0);
            assert_eq!(r.len(), n);
            assert!(innerhalb(&r), "n={n}");
            for i in 0..n {
                for j in i + 1..n {
                    let (a, b) = (r[i], r[j]);
                    let ueber = a[0] < b[0] + b[2] - 1e-6 && b[0] < a[0] + a[2] - 1e-6 && a[1] < b[1] + b[3] - 1e-6 && b[1] < a[1] + a[3] - 1e-6;
                    assert!(!ueber, "n={n} {i}/{j}");
                }
            }
        }
    }
}
