//! Umfang: Wie viele Seiten bekommt jedes Ereignis?
//!
//! Statt einer festen Zahl «Fotos pro Seite» gilt ein Bereich (von … bis). Jedes Ereignis bekommt so viele Seiten,
//! dass seine Fotos in diesen Bereich passen. Ein kleines Ereignis hat eine Seite, ein grosses mehrere, auch ungerade.
//! Doppelseiten werden bevorzugt: Ein Ereignis soll möglichst auf einer linken Seite beginnen. Das ist nur ein Wunsch,
//! die Rechnung gibt ihn zugunsten eines guten Fotoverhältnisses auf, wenn es nicht anders geht.
//! Pro Ereignis lässt sich die Dichte (kompakt, normal, gross) oder die Seitenzahl von Hand festlegen.

use crate::pages::{erlaubte_seitenzahl, Profil};

/// Eingabe je Ereignis (oder je Gruppe, wenn Ereignisse nicht getrennt werden).
#[derive(Clone, Debug, Default)]
pub struct Eingabe {
    /// Fotos, die sich Seiten teilen
    pub normal: i64,
    /// Seiten, die Fotos mit eigener Seite oder über die Doppelseite fest belegen
    pub sonder: i64,
    /// -1 kompakt, 0 normal, 1 gross
    pub dichte: i64,
    /// Von Hand festgelegte Seitenzahl für die normalen Fotos
    pub manuell: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct GruppenPlan {
    /// Seiten für die normalen Fotos
    pub normal_seiten: i64,
    pub sonder: i64,
    /// Kleinste und grösste sinnvolle Seitenzahl für die normalen Fotos
    pub min: i64,
    pub max: i64,
    /// Erste Seite des Ereignisses (1-basiert)
    pub start: i64,
    pub manuell: bool,
}

impl GruppenPlan {
    pub fn seiten(&self) -> i64 {
        self.normal_seiten + self.sonder
    }
    /// Seite 1 steht allein rechts, gerade Seiten sind links.
    pub fn beginnt_links(&self) -> bool {
        self.start % 2 == 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub gruppen: Vec<GruppenPlan>,
    pub summe: i64,
    pub buch: i64,
    pub leer: i64,
    pub kleinere: Option<i64>,
    pub groessere: Option<i64>,
}

/// Seitenzahlbereich für `n` normale Fotos bei `von` bis `bis` Fotos pro Seite.
pub fn seitenbereich(n: i64, von: i64, bis: i64) -> (i64, i64) {
    if n <= 0 {
        return (0, 0);
    }
    let (von, bis) = (von.max(1), bis.max(von.max(1)));
    let lo = ((n + bis - 1) / bis).max(1);
    let hi = (n / von).max(lo).min(n);
    (lo, hi)
}

fn ideal(e: &Eingabe, von: i64, bis: i64) -> f64 {
    let mitte = (von + bis) as f64 / 2.0;
    let dichte = match e.dichte {
        d if d < 0 => bis as f64,
        d if d > 0 => von as f64,
        _ => mitte,
    };
    e.normal as f64 / dichte
}

/// Verteilt `n` Fotos möglichst gleichmässig auf `p` Seiten (grössere Seiten zuerst).
pub fn verteile_auf(n: i64, p: i64) -> Vec<i64> {
    if n <= 0 || p <= 0 {
        return vec![];
    }
    let p = p.min(n);
    let (basis, rest) = (n / p, n % p);
    (0..p).map(|i| if i < rest { basis + 1 } else { basis }).collect()
}

const STRAFE_RECHTS: f64 = 0.6;
const STRAFE_LEER: f64 = 0.6;

/// Seitenplan für alle Ereignisse. `ziel`: gewünschte Seitenzahl (wird auf eine erlaubte Zahl gebracht).
pub fn plane(ein: &[Eingabe], von: i64, bis: i64, profil: &Profil, ziel: Option<i64>) -> Plan {
    let (von, bis) = (von.max(1), bis.max(von.max(1)));
    // Wahl je Ereignis: erlaubte Seitenzahlen der normalen Fotos
    let optionen: Vec<(i64, i64)> = ein
        .iter()
        .map(|e| match e.manuell {
            Some(m) if e.normal > 0 => {
                let m = m.clamp(1, e.normal);
                (m, m)
            }
            _ => seitenbereich(e.normal, von, bis),
        })
        .collect();
    let ideale: Vec<f64> = ein.iter().map(|e| ideal(e, von, bis)).collect();
    let basis: i64 = ein
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (lo, hi) = optionen[i];
            let p = match e.manuell {
                Some(_) => lo,
                None if e.normal > 0 => (ideale[i].round() as i64).clamp(lo, hi),
                None => 0,
            };
            p + e.sonder
        })
        .sum();
    let min_summe: i64 = ein.iter().enumerate().map(|(i, e)| optionen[i].0 + e.sonder).sum();
    let max_summe: i64 = ein.iter().enumerate().map(|(i, e)| optionen[i].1 + e.sonder).sum();
    let wunsch = ziel.map(|z| erlaubte_seitenzahl(z.max(1), profil)).unwrap_or_else(|| erlaubte_seitenzahl(basis.max(1), profil));
    // Das Ziel muss erreichbar sein: nicht unter dem Minimum, ein Überschuss bleibt leer
    let t = wunsch.max(erlaubte_seitenzahl(min_summe.max(1), profil)).max(min_summe);

    // Dynamische Programmierung über die Zahl der belegten Seiten
    let n = ein.len();
    let tmax = t as usize;
    const OFFEN: f64 = f64::INFINITY;
    let mut kosten = vec![vec![OFFEN; tmax + 1]; n + 1];
    let mut wahl = vec![vec![0i64; tmax + 1]; n + 1];
    kosten[0][0] = 0.0;
    for i in 0..n {
        let e = &ein[i];
        let (lo, hi) = optionen[i];
        for k in 0..=tmax {
            if kosten[i][k] == OFFEN {
                continue;
            }
            let start = k as i64 + 1;
            let mut p = lo;
            loop {
                let belegt = p + e.sonder;
                let nk = k + belegt as usize;
                if nk <= tmax {
                    let abw = if e.normal > 0 && e.manuell.is_none() { ((p as f64 - ideale[i]) / ideale[i].max(1.0)).powi(2) * 4.0 } else { 0.0 };
                    let strafe = if start > 1 && start % 2 == 1 && belegt >= 2 { STRAFE_RECHTS } else { 0.0 };
                    let c = kosten[i][k] + abw + strafe;
                    if c < kosten[i + 1][nk] {
                        kosten[i + 1][nk] = c;
                        wahl[i + 1][nk] = p;
                    }
                }
                if p >= hi {
                    break;
                }
                p += 1;
            }
        }
    }
    let (mut bestk, mut best) = (0usize, OFFEN);
    for k in 0..=tmax {
        let c = kosten[n][k] + (t - k as i64) as f64 * STRAFE_LEER;
        if c < best {
            best = c;
            bestk = k;
        }
    }
    let mut gruppen = vec![GruppenPlan::default(); n];
    let mut k = bestk;
    for i in (0..n).rev() {
        let p = wahl[i + 1][k];
        gruppen[i].normal_seiten = p;
        gruppen[i].sonder = ein[i].sonder;
        gruppen[i].min = optionen[i].0;
        gruppen[i].max = optionen[i].1;
        gruppen[i].manuell = ein[i].manuell.is_some() && ein[i].normal > 0;
        k -= (p + ein[i].sonder) as usize;
    }
    let mut cursor = 1;
    for g in gruppen.iter_mut() {
        g.start = cursor;
        cursor += g.seiten();
    }
    let summe = cursor - 1;
    let buch = erlaubte_seitenzahl(summe.max(1), profil);
    let schritt = profil.schritt();
    let kleinere = (buch - schritt >= profil.min() && min_summe <= buch - schritt && summe > buch - schritt).then_some(buch - schritt);
    let groessere = (buch + schritt <= profil.max() && max_summe >= buch + schritt).then_some(buch + schritt);
    Plan { gruppen, summe, buch, leer: (buch - summe).max(0), kleinere, groessere }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::lade_profil;

    fn profil() -> Profil {
        lade_profil("allgemein").unwrap().mit_vorgaben(None, None, None)
    }
    fn e(n: i64) -> Eingabe {
        Eingabe { normal: n, ..Default::default() }
    }

    #[test]
    fn bereich_statt_fester_zahl() {
        assert_eq!(seitenbereich(18, 2, 5), (4, 9));
        assert_eq!(seitenbereich(1, 2, 5), (1, 1), "ein Foto braucht eine Seite");
        assert_eq!(seitenbereich(3, 2, 5), (1, 1));
        assert_eq!(seitenbereich(0, 2, 5), (0, 0));
    }

    #[test]
    fn kleine_ereignisse_eine_seite_grosse_mehrere() {
        let p = plane(&[e(2), e(30), e(3), e(40)], 2, 5, &profil(), None);
        assert_eq!(p.gruppen[0].normal_seiten, 1);
        assert_eq!(p.gruppen[2].normal_seiten, 1);
        assert!(p.gruppen[1].normal_seiten >= 6 && p.gruppen[3].normal_seiten >= 8, "{:?}", p.gruppen);
        for g in &p.gruppen {
            assert!(g.normal_seiten >= g.min && g.normal_seiten <= g.max);
        }
        assert_eq!(p.summe, p.gruppen.iter().map(|g| g.seiten()).sum::<i64>());
        assert!(p.buch >= p.summe && p.buch % 2 == 0);
    }

    #[test]
    fn ungerade_seitenzahlen_sind_erlaubt() {
        // Ein Ereignis mit 15 Fotos hat 3 bis 7 Seiten, fünf Seiten sind möglich und werden nicht auf 4 oder 6 gezwungen
        let p = plane(&[e(15)], 3, 3, &profil(), Some(5));
        assert_eq!(p.gruppen[0].normal_seiten, 5);
    }

    #[test]
    fn doppelseiten_werden_bevorzugt() {
        // Zwei Ereignisse: das zweite soll links beginnen (gerade Seite). Das erste beginnt auf Seite 1 und hat deshalb eine ungerade Seitenzahl.
        let p = plane(&[e(14), e(14), e(14)], 2, 5, &profil(), None);
        assert!(p.gruppen[1].beginnt_links() && p.gruppen[2].beginnt_links(), "{:?}", p.gruppen);
    }

    #[test]
    fn dichte_und_handwahl() {
        let basis = plane(&[e(30), e(30)], 2, 6, &profil(), Some(24));
        let gross = plane(&[Eingabe { normal: 30, dichte: 1, ..Default::default() }, e(30)], 2, 6, &profil(), Some(24));
        assert!(gross.gruppen[0].normal_seiten >= basis.gruppen[0].normal_seiten);
        let hand = plane(&[Eingabe { normal: 30, manuell: Some(7), ..Default::default() }, e(30)], 2, 6, &profil(), None);
        assert_eq!(hand.gruppen[0].normal_seiten, 7);
        assert!(hand.gruppen[0].manuell);
    }

    #[test]
    fn zielseitenzahl_wird_erreicht_oder_geklemmt() {
        let es = [e(40), e(40), e(40), e(40)];
        let p = plane(&es, 2, 5, &profil(), Some(32));
        assert_eq!(p.buch, 32);
        assert_eq!(p.summe, 32, "ohne leere Seiten, weil genug Platz im Bereich ist");
        let zu_klein = plane(&es, 2, 5, &profil(), Some(24));
        assert!(zu_klein.summe >= 4 * 8, "unter dem Mindestumfang geht es nicht: {}", zu_klein.summe);
    }

    #[test]
    fn sonderseiten_zaehlen_mit() {
        let ein = [Eingabe { normal: 8, sonder: 3, ..Default::default() }];
        let p = plane(&ein, 2, 5, &profil(), None);
        assert_eq!(p.gruppen[0].seiten(), p.gruppen[0].normal_seiten + 3);
    }

    #[test]
    fn verteilung_ist_gleichmaessig() {
        assert_eq!(verteile_auf(7, 3), vec![3, 2, 2]);
        assert_eq!(verteile_auf(2, 5), vec![1, 1]);
        assert!(verteile_auf(0, 3).is_empty());
    }
}
