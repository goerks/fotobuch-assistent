//! Ereignisse bilden: Fotos nach Zeit sortieren und bei grossen Lücken oder Ortswechseln trennen.

use chrono::{Datelike, NaiveDateTime, Timelike, Weekday};

use crate::foto::Foto;
use crate::scanner::entfernung_km;

pub fn wochentag(z: &NaiveDateTime) -> &'static str {
    match z.weekday() {
        Weekday::Mon => "Mo",
        Weekday::Tue => "Di",
        Weekday::Wed => "Mi",
        Weekday::Thu => "Do",
        Weekday::Fri => "Fr",
        Weekday::Sat => "Sa",
        Weekday::Sun => "So",
    }
}

#[derive(Clone, Debug)]
pub struct Ereignis {
    pub nummer: usize,
    /// Indizes in die Fotoliste, nach Zeit sortiert
    pub fotos: Vec<usize>,
}

impl Ereignis {
    pub fn start(&self, fotos: &[Foto]) -> NaiveDateTime {
        fotos[self.fotos[0]].zeit
    }
    pub fn ende(&self, fotos: &[Foto]) -> NaiveDateTime {
        fotos[*self.fotos.last().unwrap()].zeit
    }
    /// Alle Fotos ohne Aufnahmezeit, während andere eine haben: sie bilden das Schluss-Ereignis «Ohne Aufnahmezeit».
    pub fn ohne_datum(&self, fotos: &[Foto]) -> bool {
        self.fotos.iter().all(|&i| fotos[i].zeit_quelle == "datei") && fotos.iter().any(|f| f.zeit_quelle != "datei")
    }
    /// Lesbarer Name aus Wochentag, Datum und Tageszeit, zum Beispiel «Sa 18.07. Nachmittag».
    pub fn titel(&self, fotos: &[Foto]) -> String {
        if self.ohne_datum(fotos) {
            return "Ohne Aufnahmezeit".to_string();
        }
        let (s, e) = (self.start(fotos), self.ende(fotos));
        if s.date() != e.date() {
            return format!("{} {} bis {} {}", wochentag(&s), s.format("%d.%m."), wochentag(&e), e.format("%d.%m."));
        }
        format!("{} {} {}", wochentag(&s), s.format("%d.%m."), tageszeit(s.hour(), e.hour()))
    }
}

/// Tageszeit eines Ereignisses nach Start- und Endstunde.
pub fn tageszeit(von: u32, bis: u32) -> &'static str {
    let teil = |h: u32| match h {
        5..=11 => 0,
        12..=16 => 1,
        17..=21 => 2,
        _ => 3,
    };
    let (a, b) = (teil(von), teil(bis));
    if a == b {
        ["Vormittag", "Nachmittag", "Abend", "Nacht"][a]
    } else if a == 0 && b >= 2 {
        "Tagsüber"
    } else if a == 0 {
        "Vormittag bis Nachmittag"
    } else if a == 1 {
        "Nachmittag und Abend"
    } else {
        "Abend"
    }
}

/// Sortiert nach Aufnahmezeit (dann Name) und trennt bei Zeitlücke oder grossem Ortssprung.
pub fn bilde_ereignisse(fotos: &[Foto], luecke_stunden: f64, max_km: f64) -> Vec<Ereignis> {
    let mut reihe: Vec<usize> = (0..fotos.len()).collect();
    // Fotos ohne Aufnahmezeit (nur Änderungsdatum) kommen ans Ende in ein eigenes Ereignis, sofern es auch Fotos mit Zeit gibt.
    let mut ohne: Vec<usize> = Vec::new();
    if fotos.iter().any(|f| f.zeit_quelle != "datei") {
        ohne = reihe.iter().copied().filter(|&i| fotos[i].zeit_quelle == "datei").collect();
        reihe.retain(|&i| fotos[i].zeit_quelle != "datei");
        ohne.sort_by_key(|&i| fotos[i].name.to_lowercase());
    }
    reihe.sort_by(|&a, &b| (fotos[a].zeit, fotos[a].name.to_lowercase()).cmp(&(fotos[b].zeit, fotos[b].name.to_lowercase())));
    let mut ereignisse: Vec<Ereignis> = Vec::new();
    let mut letzte_gps: Option<(f64, f64)> = None;
    for i in reihe {
        let f = &fotos[i];
        let mut neu = ereignisse.is_empty();
        if !neu {
            let vorher = *ereignisse.last().unwrap().fotos.last().unwrap();
            let luecke = (f.zeit - fotos[vorher].zeit).num_milliseconds() as f64 / 3_600_000.0;
            if luecke > luecke_stunden {
                neu = true;
            } else if let (Some(g), Some(l)) = (f.gps, letzte_gps) {
                if entfernung_km(g, l) > max_km {
                    neu = true;
                }
            }
        }
        if neu {
            ereignisse.push(Ereignis { nummer: ereignisse.len() + 1, fotos: vec![] });
        }
        ereignisse.last_mut().unwrap().fotos.push(i);
        if f.gps.is_some() {
            letzte_gps = f.gps;
        }
    }
    if !ohne.is_empty() {
        ereignisse.push(Ereignis { nummer: ereignisse.len() + 1, fotos: ohne });
    }
    ereignisse
}

#[cfg(test)]
mod namen_tests {
    use super::tageszeit;

    #[test]
    fn tageszeit_nach_start_und_ende() {
        assert_eq!(tageszeit(9, 11), "Vormittag");
        assert_eq!(tageszeit(14, 16), "Nachmittag");
        assert_eq!(tageszeit(18, 20), "Abend");
        assert_eq!(tageszeit(9, 19), "Tagsüber");
        assert_eq!(tageszeit(13, 19), "Nachmittag und Abend");
    }
}
