//! Ereignisse bilden: Fotos nach Zeit sortieren und bei grossen Lücken oder Ortswechseln trennen.

use chrono::{Datelike, NaiveDateTime, Weekday};

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
    pub fn titel(&self, fotos: &[Foto]) -> String {
        let (s, e) = (self.start(fotos), self.ende(fotos));
        let tag = format!("{} {}", wochentag(&s), s.format("%d.%m.%Y"));
        if s.date() == e.date() {
            format!("{} {}–{}", tag, s.format("%H:%M"), e.format("%H:%M"))
        } else {
            format!("{} {} bis {} {}", tag, s.format("%H:%M"), wochentag(&e), e.format("%d.%m.%Y %H:%M"))
        }
    }
}

/// Sortiert nach Aufnahmezeit (dann Name) und trennt bei Zeitlücke oder grossem Ortssprung.
pub fn bilde_ereignisse(fotos: &[Foto], luecke_stunden: f64, max_km: f64) -> Vec<Ereignis> {
    let mut reihe: Vec<usize> = (0..fotos.len()).collect();
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
    ereignisse
}
