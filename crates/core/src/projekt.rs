//! Projektzustand der App: Fotos, Gruppen, Entscheidungen, Seitenplan, Speicherung, Export.
//!
//! Ein Projekt gehört zu einem Fotoordner. Alles wird im Unterordner `.fotobuch` des Fotoordners
//! abgelegt (Zustand und Vorschaubilder). Die Originalfotos werden nie verändert.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Local;
use image::imageops::FilterType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};

use crate::duplicates::{finde_aehnliche, finde_exakte, Gruppe, SerienOptionen};
use crate::events::{bilde_ereignisse, Ereignis};
use crate::foto::Foto;
use crate::layout::seitenlayout;
use crate::orte::{bilde_orte, Ort};
use crate::pages::{lade_profil, seitenplan_ereignisse, seitenzuordnung, Profil, PROFILE};
use crate::scanner::{self, analysiere_foto, datei_zeiten, finde_fotos};

pub const ZUSTANDSDATEI: &str = "projekt.json";
pub const FORMAT_VERSION: i64 = 1;

pub type Res<T> = Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Einstellungen {
    pub luecke_stunden: f64,
    pub max_km: f64,
    pub aehnlich_abstand: i64,
    pub serie_minuten: f64,
    pub ort_radius_m: i64,
    pub max_pro_ort: Option<i64>,
    pub fotos_pro_seite: i64,
    pub profil: String,
    pub seiten_min: Option<i64>,
    pub seiten_schritt: Option<i64>,
    pub seiten_max: Option<i64>,
    pub ereignisse_getrennt: bool,
    pub seitenformat: String,
}

impl Default for Einstellungen {
    fn default() -> Self {
        Einstellungen {
            luecke_stunden: 6.0,
            max_km: 50.0,
            aehnlich_abstand: 6,
            serie_minuten: 15.0,
            ort_radius_m: 200,
            max_pro_ort: None,
            fotos_pro_seite: 3,
            profil: "allgemein".into(),
            seiten_min: None,
            seiten_schritt: None,
            seiten_max: None,
            ereignisse_getrennt: true,
            seitenformat: "quer".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CacheEintrag {
    mtime_ns: u64,
    groesse: u64,
    foto: Foto,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Zustand {
    version: i64,
    programmversion: String,
    einstellungen: Einstellungen,
    entscheidungen: BTreeMap<String, String>,
    ereignis_namen: BTreeMap<String, String>,
    ort_namen: BTreeMap<String, String>,
    schritt: i64,
    cache: BTreeMap<String, CacheEintrag>,
    analysiert_am: Option<String>,
    analyse_dauer: Option<f64>,
}

pub fn foto_id(rel: &str) -> String {
    let h = Sha1::digest(rel.replace('\\', "/").to_lowercase().as_bytes());
    let hex: String = h.iter().map(|b| format!("{:02x}", b)).collect();
    hex[..12].to_string()
}

fn sauber(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || (c as u32) < 0x20 { '_' } else { c })
        .collect();
    let s = s.trim_matches(|c| c == ' ' || c == '.').to_string();
    if s.is_empty() { "_".into() } else { s }
}

fn kuerze(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub struct Ergebnis {
    pub ereignisse: Vec<Ereignis>,
    pub exakt: Vec<Gruppe>,
    pub serien: Vec<Gruppe>,
    pub orte: Vec<Ort>,
    pub ort_von: Vec<Option<usize>>,
    pub ereignis_von: Vec<usize>,
    pub exakt_von: HashMap<usize, usize>,
    pub serie_von: HashMap<usize, usize>,
    pub zu_entfernen_exakt: HashSet<usize>,
    pub zu_entfernen_serien: HashSet<usize>,
    pub screenshots: HashSet<usize>,
}

#[derive(Clone, Debug)]
pub struct Einstufung {
    pub behalten: bool,
    pub grund: String,
    pub eigene: bool,
}

pub struct Projekt {
    pub ordner: PathBuf,
    pub daten: PathBuf,
    pub einstellungen: Einstellungen,
    pub entscheidungen: BTreeMap<String, String>,
    pub ereignis_namen: BTreeMap<String, String>,
    pub ort_namen: BTreeMap<String, String>,
    pub schritt: i64,
    cache: BTreeMap<String, CacheEintrag>,
    pub fotos: Vec<Foto>,
    pub ids: Vec<String>,
    id_index: HashMap<String, usize>,
    pub ergebnis: Option<Ergebnis>,
    pub analysiert_am: Option<String>,
    pub analyse_dauer: Option<f64>,
    ver: u64,
    ort_weg_cache: RefCell<Option<(u64, HashSet<usize>)>>,
    maxs: f64,
}

fn score(f: &Foto, maxs: f64) -> f64 {
    let s = if maxs > 0.0 { (f.schaerfe / maxs).min(1.0) } else { 0.0 };
    0.7 * s + 0.3 * (f.megapixel() / 12.0).min(1.0)
}

/// Lineare Interpolation wie numpy.percentile
fn perzentil(werte: &mut Vec<f64>, p: f64) -> f64 {
    if werte.is_empty() {
        return 0.0;
    }
    werte.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pos = (werte.len() - 1) as f64 * p / 100.0;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    werte[lo] + (werte[hi] - werte[lo]) * (pos - lo as f64)
}

fn zahl_aus(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        Value::Bool(b) => Some(*b as i64 as f64),
        _ => None,
    }
}

fn ganzzahl_aus(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

impl Projekt {
    pub fn neu(ordner: &Path) -> Res<Projekt> {
        let ordner = std::fs::canonicalize(ordner).map_err(|_| format!("Der Ordner existiert nicht: {}", ordner.display()))?;
        if !ordner.is_dir() {
            return Err(format!("Der Ordner existiert nicht: {}", ordner.display()));
        }
        let mut p = Projekt {
            daten: ordner.join(".fotobuch"),
            ordner,
            einstellungen: Einstellungen::default(),
            entscheidungen: BTreeMap::new(),
            ereignis_namen: BTreeMap::new(),
            ort_namen: BTreeMap::new(),
            schritt: 1,
            cache: BTreeMap::new(),
            fotos: vec![],
            ids: vec![],
            id_index: HashMap::new(),
            ergebnis: None,
            analysiert_am: None,
            analyse_dauer: None,
            ver: 0,
            ort_weg_cache: RefCell::new(None),
            maxs: 0.0,
        };
        p.laden();
        if !p.cache.is_empty() {
            p.fotos_aus_cache();
            p.neu_gruppieren();
        }
        Ok(p)
    }

    // ------------------------------------------------------------------ Speicherung

    fn laden(&mut self) {
        let pfad = self.daten.join(ZUSTANDSDATEI);
        let Ok(text) = std::fs::read_to_string(&pfad) else { return };
        match serde_json::from_str::<Zustand>(&text) {
            Ok(z) => {
                if z.version != FORMAT_VERSION {
                    return;
                }
                self.einstellungen = z.einstellungen;
                self.entscheidungen = z.entscheidungen;
                self.ereignis_namen = z.ereignis_namen;
                self.ort_namen = z.ort_namen;
                self.schritt = if z.schritt == 0 { 1 } else { z.schritt };
                self.cache = z.cache;
                self.analysiert_am = z.analysiert_am;
                self.analyse_dauer = z.analyse_dauer;
            }
            Err(_) => {
                // Defekte Datei nicht löschen, sondern beiseite legen
                let _ = std::fs::rename(&pfad, pfad.with_extension("defekt"));
            }
        }
    }

    pub fn speichern(&mut self) {
        self.ver += 1;
        let z = Zustand {
            version: FORMAT_VERSION,
            programmversion: crate::VERSION.to_string(),
            einstellungen: self.einstellungen.clone(),
            entscheidungen: self.entscheidungen.clone(),
            ereignis_namen: self.ereignis_namen.clone(),
            ort_namen: self.ort_namen.clone(),
            schritt: self.schritt,
            cache: self.cache.clone(),
            analysiert_am: self.analysiert_am.clone(),
            analyse_dauer: self.analyse_dauer,
        };
        if std::fs::create_dir_all(&self.daten).is_err() {
            return;
        }
        let ziel = self.daten.join(ZUSTANDSDATEI);
        let tmp = ziel.with_extension("tmp");
        if let Ok(text) = serde_json::to_string(&z) {
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &ziel);
            }
        }
    }

    // ------------------------------------------------------------------ Analyse

    fn fotos_aus_cache(&mut self) {
        let mut eintraege: Vec<(&String, &CacheEintrag)> = self.cache.iter().collect();
        eintraege.sort_by_key(|(rel, _)| rel.to_lowercase());
        self.fotos = eintraege.iter().map(|(_, e)| e.foto.clone()).collect();
        self.ids = eintraege.iter().map(|(rel, _)| foto_id(rel)).collect();
        self.id_index = self.ids.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
    }

    // ------------------------------------------------------------------ Gruppierung

    pub fn profil(&self) -> Profil {
        let e = &self.einstellungen;
        let basis = lade_profil(&e.profil).or_else(|| lade_profil("allgemein")).unwrap();
        basis.mit_vorgaben(e.seiten_min, e.seiten_schritt, e.seiten_max)
    }

    pub fn neu_gruppieren(&mut self) {
        let e = &self.einstellungen;
        let fotos = &self.fotos;
        let ereignisse = bilde_ereignisse(fotos, e.luecke_stunden, e.max_km);
        let alle: Vec<usize> = (0..fotos.len()).collect();
        let exakt = finde_exakte(fotos);
        let (orte, ort_von) = bilde_orte(fotos, e.ort_radius_m as f64, 30.0);
        let kandidaten: Vec<usize> = alle.iter().copied().filter(|&i| !fotos[i].screenshot_verdacht).collect();
        let serien = finde_aehnliche(
            fotos,
            &kandidaten,
            &SerienOptionen {
                max_abstand: e.aehnlich_abstand as u32,
                fenster_sekunden: e.serie_minuten * 60.0,
                max_spanne_sekunden: None,
                fenster_ort_sekunden: 3600.0,
            },
            &ort_von,
        );
        let mut zu_exakt = HashSet::new();
        for g in &exakt {
            zu_exakt.extend(g.fotos.iter().copied().filter(|&i| i != g.vorschlag));
        }
        let mut zu_serien = HashSet::new();
        for g in &serien {
            zu_serien.extend(g.fotos.iter().copied().filter(|&i| i != g.vorschlag));
        }
        let mut exakt_von = HashMap::new();
        for (k, g) in exakt.iter().enumerate() {
            for &i in &g.fotos {
                exakt_von.insert(i, k);
            }
        }
        let mut serie_von = HashMap::new();
        for (k, g) in serien.iter().enumerate() {
            for &i in &g.fotos {
                serie_von.insert(i, k);
            }
        }
        let mut ereignis_von = vec![0usize; fotos.len()];
        for (k, ev) in ereignisse.iter().enumerate() {
            for &i in &ev.fotos {
                ereignis_von[i] = k;
            }
        }
        let screenshots = alle.iter().copied().filter(|&i| fotos[i].screenshot_verdacht).collect();
        let mut vals: Vec<f64> = fotos.iter().filter(|f| f.fehler.is_empty() && f.schaerfe > 0.0).map(|f| f.schaerfe).collect();
        self.maxs = perzentil(&mut vals, 98.0);
        self.ergebnis = Some(Ergebnis {
            ereignisse,
            exakt,
            serien,
            orte,
            ort_von,
            ereignis_von,
            exakt_von,
            serie_von,
            zu_entfernen_exakt: zu_exakt,
            zu_entfernen_serien: zu_serien,
            screenshots,
        });
        self.ver += 1;
    }

    fn erg(&self) -> &Ergebnis {
        self.ergebnis.as_ref().expect("noch nicht analysiert")
    }

    // ------------------------------------------------------------------ Entscheidungen

    fn auto(&self, i: usize) -> (bool, &'static str) {
        let (f, erg) = (&self.fotos[i], self.erg());
        if !f.fehler.is_empty() {
            return (false, "Nicht lesbar");
        }
        if erg.zu_entfernen_exakt.contains(&i) {
            return (false, "Exaktes Duplikat");
        }
        if f.screenshot_verdacht {
            return (false, "Screenshot");
        }
        if erg.zu_entfernen_serien.contains(&i) {
            return (false, "Serie, anderes Foto vorgeschlagen");
        }
        (true, "")
    }

    /// Fotos, die wegen «höchstens N Fotos pro Ort» automatisch entfallen (die schwächsten zuerst).
    fn ort_weg(&self) -> HashSet<usize> {
        let Some(n) = self.einstellungen.max_pro_ort.filter(|&n| n > 0) else { return HashSet::new() };
        if let Some((v, s)) = &*self.ort_weg_cache.borrow() {
            if *v == self.ver {
                return s.clone();
            }
        }
        let mut weg = HashSet::new();
        for ort in &self.erg().orte {
            let mut kandidaten: Vec<usize> = Vec::new();
            for &i in &ort.fotos {
                let eig = self.entscheidungen.get(&self.ids[i]).map(|s| s.as_str());
                if eig == Some("entfernen") || !self.auto(i).0 {
                    continue;
                }
                if eig != Some("behalten") {
                    kandidaten.push(i); // eigene Auswahl liegt ausserhalb der Begrenzung
                }
            }
            kandidaten.sort_by(|&a, &b| {
                let (sa, sb) = (score(&self.fotos[a], self.maxs), score(&self.fotos[b], self.maxs));
                sb.partial_cmp(&sa).unwrap().then_with(|| self.fotos[a].name.to_lowercase().cmp(&self.fotos[b].name.to_lowercase()))
            });
            weg.extend(kandidaten.into_iter().skip(n as usize));
        }
        *self.ort_weg_cache.borrow_mut() = Some((self.ver, weg.clone()));
        weg
    }

    pub fn einstufung(&self, i: usize) -> Einstufung {
        let (mut behalten, mut grund) = {
            let (b, g) = self.auto(i);
            (b, g.to_string())
        };
        if behalten && self.ort_weg().contains(&i) {
            behalten = false;
            grund = format!("Ort: nur die besten {}", self.einstellungen.max_pro_ort.unwrap_or(0));
        }
        match self.entscheidungen.get(&self.ids[i]).map(|s| s.as_str()) {
            Some("behalten") => Einstufung { behalten: true, grund: "Von dir behalten".into(), eigene: true },
            Some("entfernen") => Einstufung { behalten: false, grund: "Von dir entfernt".into(), eigene: true },
            _ => Einstufung { behalten, grund, eigene: false },
        }
    }

    fn einstufungen(&self) -> Vec<Einstufung> {
        // Die Ortsbegrenzung wird einmal berechnet und dann für alle Fotos wiederverwendet.
        (0..self.fotos.len()).map(|i| self.einstufung(i)).collect()
    }

    #[doc(hidden)]
    pub fn setze_foto_gueltig(&self, id: &str) -> Res<()> {
        if self.id_index.contains_key(id) { Ok(()) } else { Err("Unbekanntes Foto".into()) }
    }

    pub fn setze_foto(&mut self, id: &str, aktion: &str) -> Res<()> {
        if !self.id_index.contains_key(id) {
            return Err("Unbekanntes Foto".into());
        }
        match aktion {
            "auto" => {
                self.entscheidungen.remove(id);
            }
            "behalten" | "entfernen" => {
                self.entscheidungen.insert(id.to_string(), aktion.to_string());
            }
            _ => return Err("Unbekannte Aktion".into()),
        }
        self.speichern();
        Ok(())
    }

    /// `waehle`: Foto-ID, Liste von IDs, "vorschlag" (zurück zum Vorschlag) oder "keines".
    pub fn serie_entscheiden(&mut self, nr: i64, waehle: &Value) -> Res<()> {
        let erg = self.ergebnis.as_ref().ok_or("Noch nicht analysiert")?;
        if nr < 0 || nr as usize >= erg.serien.len() {
            return Err("Unbekannte Serie".into());
        }
        let ids: Vec<String> = erg.serien[nr as usize].fotos.iter().map(|&i| self.ids[i].clone()).collect();
        match waehle {
            Value::String(s) if s == "vorschlag" => {
                for i in &ids {
                    self.entscheidungen.remove(i);
                }
            }
            Value::String(s) if s == "keines" => {
                for i in &ids {
                    self.entscheidungen.insert(i.clone(), "entfernen".into());
                }
            }
            v => {
                let gewaehlt: Vec<String> = match v {
                    Value::String(s) => vec![s.clone()],
                    Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(),
                    _ => vec![],
                };
                if gewaehlt.is_empty() || gewaehlt.iter().any(|x| !ids.contains(x)) {
                    return Err("Die Auswahl gehört nicht zu dieser Serie".into());
                }
                for i in &ids {
                    let w = if gewaehlt.contains(i) { "behalten" } else { "entfernen" };
                    self.entscheidungen.insert(i.clone(), w.into());
                }
            }
        }
        self.speichern();
        Ok(())
    }

    pub fn alle_serien_auf_vorschlag(&mut self) {
        let ids: Vec<String> = self.erg().serien.iter().flat_map(|g| g.fotos.iter().map(|&i| self.ids[i].clone())).collect();
        for i in ids {
            self.entscheidungen.remove(&i);
        }
        self.speichern();
    }

    pub fn alle_entscheidungen_zuruecksetzen(&mut self) {
        self.entscheidungen.clear();
        self.speichern();
    }

    pub fn ereignis_umbenennen(&mut self, key: &str, name: &str) {
        let name = kuerze(name.trim(), 80);
        if name.is_empty() {
            self.ereignis_namen.remove(key);
        } else {
            self.ereignis_namen.insert(key.to_string(), name);
        }
        self.speichern();
    }

    pub fn ort_umbenennen(&mut self, key: &str, name: &str) {
        let name = kuerze(name.trim(), 80);
        if name.is_empty() {
            self.ort_namen.remove(key);
        } else {
            self.ort_namen.insert(key.to_string(), name);
        }
        self.speichern();
    }

    pub fn ort_key(&self, ort_index: usize) -> String {
        self.ids[self.erg().orte[ort_index].fotos[0]].clone()
    }

    pub fn ort_name(&self, ort_index: usize) -> String {
        let o = &self.erg().orte[ort_index];
        self.ort_namen.get(&self.ort_key(ort_index)).cloned().unwrap_or_else(|| format!("Ort {}", o.nummer))
    }

    pub fn setze_einstellungen(&mut self, neu: &Map<String, Value>) -> Res<()> {
        for (k, v) in neu {
            let e = &mut self.einstellungen;
            match k.as_str() {
                "luecke_stunden" | "serie_minuten" | "max_km" => {
                    let (lo, hi) = match k.as_str() {
                        "luecke_stunden" => (0.5, 72.0),
                        "serie_minuten" => (1.0, 120.0),
                        _ => (1.0, 5000.0),
                    };
                    let x = zahl_aus(v).ok_or_else(|| format!("Ungültiger Wert für {k}"))?;
                    if !(lo..=hi).contains(&x) {
                        return Err(format!("{k} muss zwischen {lo} und {hi} liegen"));
                    }
                    match k.as_str() {
                        "luecke_stunden" => e.luecke_stunden = x,
                        "serie_minuten" => e.serie_minuten = x,
                        _ => e.max_km = x,
                    }
                }
                "aehnlich_abstand" | "ort_radius_m" | "fotos_pro_seite" => {
                    let (lo, hi) = match k.as_str() {
                        "aehnlich_abstand" => (0, 16),
                        "ort_radius_m" => (25, 5000),
                        _ => (1, 8),
                    };
                    let x = ganzzahl_aus(v).ok_or_else(|| format!("Ungültiger Wert für {k}"))?;
                    if !(lo..=hi).contains(&x) {
                        return Err(format!("{k} muss zwischen {lo} und {hi} liegen"));
                    }
                    match k.as_str() {
                        "aehnlich_abstand" => e.aehnlich_abstand = x,
                        "ort_radius_m" => e.ort_radius_m = x,
                        _ => e.fotos_pro_seite = x,
                    }
                }
                "profil" => {
                    let n = v.as_str().unwrap_or("");
                    if !PROFILE.contains(&n) {
                        return Err("Unbekanntes Profil".into());
                    }
                    e.profil = n.to_string();
                }
                "seitenformat" => {
                    let n = v.as_str().unwrap_or("");
                    if !["quer", "quadrat", "hoch"].contains(&n) {
                        return Err("Unbekanntes Seitenformat".into());
                    }
                    e.seitenformat = n.to_string();
                }
                "ereignisse_getrennt" => {
                    e.ereignisse_getrennt = match v {
                        Value::Bool(b) => *b,
                        Value::Null => false,
                        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                        Value::String(s) => !s.is_empty(),
                        _ => true,
                    }
                }
                "seiten_min" | "seiten_schritt" | "seiten_max" | "max_pro_ort" => {
                    let x = match v {
                        Value::Null => None,
                        Value::String(s) if s.is_empty() => None,
                        _ => {
                            let x = ganzzahl_aus(v).ok_or_else(|| format!("Ungültiger Wert für {k}"))?;
                            if !(1..=1000).contains(&x) {
                                return Err(format!("{k} ist unrealistisch"));
                            }
                            Some(x)
                        }
                    };
                    match k.as_str() {
                        "seiten_min" => e.seiten_min = x,
                        "seiten_schritt" => e.seiten_schritt = x,
                        "seiten_max" => e.seiten_max = x,
                        _ => e.max_pro_ort = x,
                    }
                }
                _ => return Err(format!("Unbekannte Einstellung: {k}")),
            }
        }
        if neu.get("profil").map_or(false, |v| v.as_str().map_or(false, |s| !s.is_empty())) {
            // Beim Profilwechsel eigene Seitenregeln zurücksetzen
            let e = &mut self.einstellungen;
            if !neu.contains_key("seiten_min") {
                e.seiten_min = None;
            }
            if !neu.contains_key("seiten_schritt") {
                e.seiten_schritt = None;
            }
            if !neu.contains_key("seiten_max") {
                e.seiten_max = None;
            }
        }
        if self.ergebnis.is_some() {
            self.neu_gruppieren();
        }
        self.speichern();
        Ok(())
    }

    pub fn setze_schritt(&mut self, schritt: i64) {
        self.schritt = schritt.clamp(1, 6);
        self.speichern();
    }

    // ------------------------------------------------------------------ Auswertung

    /// Je Ereignis: (alle Fotos, Fotos im Buch)
    fn ereignis_zahlen(&self, einst: &[Einstufung]) -> Vec<(Vec<usize>, Vec<usize>)> {
        self.erg()
            .ereignisse
            .iter()
            .map(|ev| {
                let buch = ev.fotos.iter().copied().filter(|&i| einst[i].behalten).collect();
                (ev.fotos.clone(), buch)
            })
            .collect()
    }

    pub fn seitenplan_aus(&self, zahlen: &[(Vec<usize>, Vec<usize>)]) -> Value {
        let counts: Vec<i64> = zahlen.iter().map(|(_, b)| b.len() as i64).collect();
        seitenplan_ereignisse(&counts, self.einstellungen.fotos_pro_seite, &self.profil(), self.einstellungen.ereignisse_getrennt)
    }

    pub fn seitenplan(&self) -> Value {
        let einst = self.einstufungen();
        self.seitenplan_aus(&self.ereignis_zahlen(&einst))
    }

    /// Konkrete Fotos zum Auffüllen oder Kürzen, passend zum Seitenzähler.
    fn vorschlaege_aus(&self, plan: &Value, einst: &[Einstufung], zahlen: &[(Vec<usize>, Vec<usize>)]) -> (Vec<usize>, Vec<usize>) {
        let fps = self.einstellungen.fotos_pro_seite.max(1);
        let getrennt = self.einstellungen.ereignisse_getrennt;
        let maxs = self.maxs;

        let mut auf: Vec<usize> = Vec::new();
        let fehlt = plan["fehlende_fotos"].as_i64().unwrap_or(0);
        if fehlt > 0 {
            let mut frei: Vec<i64> = if getrennt {
                zahlen
                    .iter()
                    .map(|(_, b)| {
                        let n = b.len() as i64;
                        if n == 0 { 0 } else { ((n + fps - 1) / fps * fps - n).max(0) }
                    })
                    .collect()
            } else {
                vec![fehlt]
            };
            let mut kandidaten: Vec<(f64, String, usize, usize)> = Vec::new();
            for (ei, (ids, _)) in zahlen.iter().enumerate() {
                for &i in ids {
                    let e = &einst[i];
                    if e.behalten || e.eigene {
                        continue;
                    }
                    if !(e.grund.starts_with("Serie") || e.grund.starts_with("Ort")) {
                        continue;
                    }
                    kandidaten.push((-score(&self.fotos[i], maxs), self.ids[i].clone(), if getrennt { ei } else { 0 }, i));
                }
            }
            kandidaten.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then_with(|| a.1.cmp(&b.1)));
            for (_s, _id, ei, i) in kandidaten {
                if auf.len() as i64 >= fehlt {
                    break;
                }
                if frei[ei] > 0 {
                    frei[ei] -= 1;
                    auf.push(i);
                }
            }
        }

        let mut kuerzen: Vec<usize> = Vec::new();
        let weglassen = plan["weglassen_fuer_kleinere_stufe"].as_i64();
        if let (Some(w), Some(kp)) = (weglassen, plan["kuerzungsplan"].as_object()) {
            if w > 0 {
                // Reihenfolge: nach Ereignis-Index, wie die Schlüssel im Plan stehen
                let mut eintraege: Vec<(usize, i64)> = kp.iter().filter_map(|(k, v)| Some((k.parse().ok()?, v.as_i64()?))).collect();
                eintraege.sort();
                for (ei, anzahl) in eintraege {
                    let buch: Vec<usize> = if getrennt { zahlen[ei].1.clone() } else { zahlen.iter().flat_map(|(_, b)| b.iter().copied()).collect() };
                    // Eigene Entscheidungen zum Behalten sind geschützt
                    let mut frei_zum_entfernen: Vec<usize> =
                        buch.into_iter().filter(|&i| self.entscheidungen.get(&self.ids[i]).map(|s| s.as_str()) != Some("behalten")).collect();
                    frei_zum_entfernen.sort_by(|&a, &b| score(&self.fotos[a], maxs).partial_cmp(&score(&self.fotos[b], maxs)).unwrap());
                    kuerzen.extend(frei_zum_entfernen.into_iter().take(anzahl as usize));
                }
                if kuerzen.len() as i64 != w {
                    kuerzen.clear(); // nicht exakt machbar, lieber nichts vorschlagen
                }
            }
        }
        (auf, kuerzen)
    }

    pub fn vorschlag_anwenden(&mut self, art: &str, ids: Option<Vec<String>>) -> Res<usize> {
        let einst = self.einstufungen();
        let zahlen = self.ereignis_zahlen(&einst);
        let plan = self.seitenplan_aus(&zahlen);
        let (auf, kuerzen) = self.vorschlaege_aus(&plan, &einst, &zahlen);
        let liste = match art {
            "auffuellen" => auf,
            "kuerzen" => kuerzen,
            _ => return Err("Unbekannter Vorschlag".into()),
        };
        let erlaubt: Vec<String> = liste.iter().map(|&i| self.ids[i].clone()).collect();
        let ziel: Vec<String> = match ids {
            Some(l) => l.into_iter().filter(|i| erlaubt.contains(i)).collect(),
            None => erlaubt,
        };
        for i in &ziel {
            self.entscheidungen.insert(i.clone(), if art == "auffuellen" { "behalten" } else { "entfernen" }.into());
        }
        self.speichern();
        Ok(ziel.len())
    }

    // ------------------------------------------------------------------ Ansicht

    fn orte_im_ereignis(&self, ids: &[usize]) -> Value {
        let mut zaehler: BTreeMap<usize, i64> = BTreeMap::new();
        for &i in ids {
            if let Some(o) = self.erg().ort_von[i] {
                *zaehler.entry(o).or_insert(0) += 1;
            }
        }
        let mut liste: Vec<(usize, i64)> = zaehler.into_iter().collect();
        liste.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        Value::Array(liste.into_iter().map(|(o, n)| json!([o, self.ort_name(o), n])).collect())
    }

    fn orte_ansicht(&self, einst: &[Einstufung]) -> (Vec<Value>, HashMap<usize, usize>) {
        let mut rang: HashMap<usize, usize> = HashMap::new();
        let mut orte: Vec<(usize, Value)> = Vec::new();
        let erg = self.erg();
        for (idx, o) in erg.orte.iter().enumerate() {
            let mut geordnet = o.fotos.clone();
            geordnet.sort_by(|&a, &b| {
                let (sa, sb) = (score(&self.fotos[a], self.maxs), score(&self.fotos[b], self.maxs));
                sb.partial_cmp(&sa).unwrap().then_with(|| self.fotos[a].name.to_lowercase().cmp(&self.fotos[b].name.to_lowercase()))
            });
            for (r, &i) in geordnet.iter().enumerate() {
                rang.insert(i, r + 1);
            }
            let behalten = o.fotos.iter().filter(|&&i| einst[i].behalten).count();
            let mut evs: Vec<usize> = o.fotos.iter().map(|&i| erg.ereignis_von[i]).collect();
            evs.sort();
            evs.dedup();
            let start = o.fotos.iter().map(|&i| self.fotos[i].zeit).min().unwrap();
            let ende = o.fotos.iter().map(|&i| self.fotos[i].zeit).max().unwrap();
            let key = self.ort_key(idx);
            let name = self.ort_namen.get(&key).cloned().unwrap_or_default();
            let ids = |l: &[usize]| -> Vec<String> { l.iter().map(|&i| self.ids[i].clone()).collect() };
            orte.push((
                o.fotos.len(),
                json!({
                    "index": idx,
                    "key": key,
                    "nummer": o.nummer,
                    "name": name,
                    "standardname": format!("Ort {}", o.nummer),
                    "lat": (o.lat * 1e5).round() / 1e5,
                    "lon": (o.lon * 1e5).round() / 1e5,
                    "anzahl": o.fotos.len(),
                    "mit_gps": o.mit_gps,
                    "im_buch": behalten,
                    "von": start.format("%Y-%m-%dT%H:%M").to_string(),
                    "bis": ende.format("%Y-%m-%dT%H:%M").to_string(),
                    "ereignisse": evs,
                    "beste": ids(&geordnet[..geordnet.len().min(8)]),
                    "alle": ids(&geordnet),
                }),
            ));
        }
        orte.sort_by_key(|(n, _)| std::cmp::Reverse(*n)); // stabil: gleiche Grösse behält die Zeitreihenfolge
        (orte.into_iter().map(|(_, v)| v).collect(), rang)
    }

    fn seiten_verhaeltnis(&self) -> f64 {
        match self.einstellungen.seitenformat.as_str() {
            "quadrat" => 1.0,
            "hoch" => 3.0 / 4.0,
            _ => 4.0 / 3.0,
        }
    }

    fn ereignis_anzeigename(&self, ei: usize) -> String {
        let ev = &self.erg().ereignisse[ei];
        let key = self.ids[ev.fotos[0]].clone();
        self.ereignis_namen.get(&key).cloned().unwrap_or_else(|| ev.titel(&self.fotos))
    }

    /// Verständliche Begründung, warum ein Foto im Buch ist oder nicht.
    fn begruendung(&self, i: usize, e: &Einstufung, rang: Option<usize>) -> String {
        let (f, erg) = (&self.fotos[i], self.erg());
        let q = |x: usize| (100.0 * score(&self.fotos[x], self.maxs)).round() as i64;
        if e.eigene {
            return if e.behalten {
                "Von dir behalten. Deine Entscheidung gilt vor allen Vorschlägen.".into()
            } else {
                "Von dir entfernt. Mit «Ins Buch» holst du es zurück.".into()
            };
        }
        if !f.fehler.is_empty() {
            return format!("Datei nicht lesbar ({}). Sie kann nicht ins Buch.", f.fehler);
        }
        if erg.zu_entfernen_exakt.contains(&i) {
            let g = &erg.exakt[erg.exakt_von[&i]];
            return format!("Exaktes Duplikat: Die Datei ist byte-genau gleich wie «{}». Diese Kopie bleibt draussen.", self.fotos[g.vorschlag].name);
        }
        if f.screenshot_verdacht {
            return "Sieht wie ein Bildschirmfoto aus (keine Kameradaten, Bildschirmformat). Du kannst es zurückholen.".into();
        }
        if erg.zu_entfernen_serien.contains(&i) {
            let g = &erg.serien[erg.serie_von[&i]];
            let b = &self.fotos[g.vorschlag];
            let mut gruende = vec![];
            if b.schaerfe > f.schaerfe * 1.05 {
                gruende.push("schärfer");
            }
            if b.megapixel() > f.megapixel() * 1.05 {
                gruende.push("höher aufgelöst");
            }
            let warum = if gruende.is_empty() { "insgesamt besser bewertet".to_string() } else { gruende.join(" und ") };
            return format!(
                "Fast gleiche Aufnahme in einer Serie von {} Fotos. «{}» ist {} (Qualität {} statt {}) und kommt ins Buch.",
                g.fotos.len(), b.name, warum, q(g.vorschlag), q(i)
            );
        }
        if e.grund.starts_with("Ort:") {
            let ort = erg.ort_von[i].map(|o| self.ort_name(o)).unwrap_or_default();
            return format!(
                "Am Ort «{}» sind höchstens {} Fotos erlaubt. Dieses liegt auf Rang {} (Qualität {}) und ist schwächer als die besten.",
                ort, self.einstellungen.max_pro_ort.unwrap_or(0), rang.unwrap_or(0), q(i)
            );
        }
        if let Some(&k) = erg.serie_von.get(&i) {
            if erg.serien[k].vorschlag == i {
                return format!("Bestes Foto einer Serie von {} ähnlichen (Qualität {}: Schärfe und Auflösung).", erg.serien[k].fotos.len(), q(i));
            }
            return "Aus einer Serie, du hast dieses Foto behalten.".into();
        }
        format!("Im Buch. Einzelfoto, keine Duplikate (Qualität {}).", q(i))
    }

    /// Seiten mit automatischem Layout. Die Fotos jedes Ereignisses folgen der Aufnahmezeit.
    fn buch_seiten(&self, zahlen: &[(Vec<usize>, Vec<usize>)], plan: &Value) -> Value {
        let fps = self.einstellungen.fotos_pro_seite.max(1);
        let getrennt = self.einstellungen.ereignisse_getrennt;
        let counts: Vec<i64> = zahlen.iter().map(|(_, b)| b.len() as i64).collect();
        let zuordnung = seitenzuordnung(&counts, fps, getrennt);
        let ar_seite = self.seiten_verhaeltnis();
        let mut warteschlangen: Vec<std::collections::VecDeque<usize>> = if getrennt {
            zahlen.iter().map(|(_, b)| b.iter().copied().collect()).collect()
        } else {
            vec![zahlen.iter().flat_map(|(_, b)| b.iter().copied()).collect()]
        };
        let erg = self.erg();
        let mut seiten = Vec::new();
        for (nr, (ei, n)) in zuordnung.iter().enumerate() {
            let q = if getrennt { &mut warteschlangen[*ei as usize] } else { &mut warteschlangen[0] };
            let fotos: Vec<usize> = (0..*n).filter_map(|_| q.pop_front()).collect();
            let ereignis = if getrennt { *ei as usize } else { erg.ereignis_von[fotos[0]] };
            let ars: Vec<f64> = fotos.iter().map(|&i| self.fotos[i].breite.max(1) as f64 / self.fotos[i].hoehe.max(1) as f64).collect();
            let felder = seitenlayout(&ars, ar_seite);
            let r = |x: f64| (x * 100.0).round() / 100.0;
            seiten.push(json!({
                "nr": nr + 1,
                "ereignis": ereignis,
                "titel": self.ereignis_anzeigename(ereignis),
                "fotos": fotos.iter().zip(felder.iter()).map(|(&i, f)| json!({"id": self.ids[i], "x": r(f[0]), "y": r(f[1]), "w": r(f[2]), "h": r(f[3])})).collect::<Vec<_>>(),
            }));
        }
        let gesamt = plan["buch_seiten"].as_i64().unwrap_or(0) as usize;
        json!({
            "format": self.einstellungen.seitenformat,
            "ar": ar_seite,
            "seiten": seiten,
            "freie_seiten": (zuordnung.len() + 1..=gesamt).collect::<Vec<_>>(),
        })
    }

    pub fn ansicht(&self) -> Value {
        if self.ergebnis.is_none() || self.fotos.is_empty() {
            return json!({
                "programmversion": crate::VERSION,
                "ordner": self.ordner.to_string_lossy(),
                "name": self.ordner.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                "schritt": 1,
                "analysiert": false,
                "leer": self.analysiert_am.is_some(),
                "einstellungen": self.einstellungen,
            });
        }
        let erg = self.erg();
        let lesbar = self.fotos.iter().filter(|f| f.fehler.is_empty()).count();
        let mut endungen: BTreeMap<&str, i64> = BTreeMap::new();
        for f in &self.fotos {
            *endungen.entry(f.endung.as_str()).or_insert(0) += 1;
        }
        let einst = self.einstufungen();
        let im_buch = einst.iter().filter(|e| e.behalten).count();
        let zahlen = self.ereignis_zahlen(&einst);
        let seitenplan = self.seitenplan_aus(&zahlen);
        let (auf, kuerzen) = self.vorschlaege_aus(&seitenplan, &einst, &zahlen);
        let profil = self.profil();
        let idl = |l: &[usize]| -> Vec<String> { l.iter().map(|&i| self.ids[i].clone()).collect() };

        let mut ereignisse = Vec::new();
        for (i, ev) in erg.ereignisse.iter().enumerate() {
            let key = self.ids[ev.fotos[0]].clone();
            let behalten: Vec<usize> = ev.fotos.iter().copied().filter(|&x| einst[x].behalten).collect();
            let basis = if behalten.is_empty() { &ev.fotos } else { &behalten };
            let schritt = (basis.len() / 8).max(1);
            let vorschau: Vec<usize> = basis.iter().copied().step_by(schritt).take(8).collect();
            ereignisse.push(json!({
                "index": i,
                "key": key,
                "nummer": ev.nummer,
                "name": self.ereignis_namen.get(&key).cloned().unwrap_or_default(),
                "titel": ev.titel(&self.fotos),
                "von": ev.start(&self.fotos).format("%Y-%m-%dT%H:%M").to_string(),
                "bis": ev.ende(&self.fotos).format("%Y-%m-%dT%H:%M").to_string(),
                "anzahl": ev.fotos.len(),
                "im_buch": behalten.len(),
                "vorschau": idl(&vorschau),
                "ohne_zeit": ev.fotos.iter().filter(|&&x| self.fotos[x].zeit_quelle == "datei").count(),
                "seiten": seitenplan["seiten_je_ereignis"][i],
                "orte": self.orte_im_ereignis(&ev.fotos),
            }));
        }

        let mut serien = Vec::new();
        let mut serien_entschieden = 0;
        for (nr, g) in erg.serien.iter().enumerate() {
            let entschieden = g.fotos.iter().any(|&i| self.entscheidungen.contains_key(&self.ids[i]));
            if entschieden {
                serien_entschieden += 1;
            }
            let beh: Vec<usize> = g.fotos.iter().copied().filter(|&i| einst[i].behalten).collect();
            serien.push(json!({
                "nr": nr,
                "ids": idl(&g.fotos),
                "vorschlag": self.ids[g.vorschlag],
                "entschieden": entschieden,
                "behalten": idl(&beh),
                "ereignis": erg.ereignis_von[g.fotos[0]],
            }));
        }

        let (orte_ansicht, rang) = self.orte_ansicht(&einst);
        let mut fotos_kompakt = Map::new();
        for (i, f) in self.fotos.iter().enumerate() {
            let e = &einst[i];
            fotos_kompakt.insert(
                self.ids[i].clone(),
                json!({
                    "n": f.name,
                    "z": f.zeit.format("%d.%m.%Y %H:%M").to_string(),
                    "f": f.format(),
                    "mp": (f.megapixel() * 10.0).round() / 10.0,
                    "b": e.behalten,
                    "g": e.grund,
                    "e": e.eigene,
                    "ev": erg.ereignis_von[i],
                    "x": f.fehler,
                    "zq": f.zeit_quelle,
                    "raw": f.ist_raw,
                    "o": erg.ort_von[i],
                    "q": (100.0 * score(f, self.maxs)).round() as i64,
                    "r": rang.get(&i),
                    "gd": self.begruendung(i, e, rang.get(&i).copied()),
                    "ort": erg.ort_von[i].map(|o| self.ort_name(o)),
                    "ar": (f.breite.max(1) as f64 / f.hoehe.max(1) as f64 * 1000.0).round() / 1000.0,
                }),
            );
        }

        let mut entfernt_gruende: BTreeMap<String, i64> = BTreeMap::new();
        for e in &einst {
            if !e.behalten {
                *entfernt_gruende.entry(e.grund.split(',').next().unwrap_or("").to_string()).or_insert(0) += 1;
            }
        }
        let mit_gps = self.fotos.iter().filter(|f| f.fehler.is_empty() && f.gps.is_some()).count();
        let ohne_ort = (0..self.fotos.len()).filter(|&i| erg.ort_von[i].is_none() && self.fotos[i].fehler.is_empty() && !self.fotos[i].screenshot_verdacht).count();
        let ort_entfernt = einst.iter().filter(|e| e.grund.starts_with("Ort:")).count();

        json!({
            "programmversion": crate::VERSION,
            "ordner": self.ordner.to_string_lossy(),
            "name": self.ordner.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            "schritt": self.schritt,
            "analysiert": self.analysiert_am.is_some() && !self.fotos.is_empty(),
            "analysiert_am": self.analysiert_am,
            "analyse_dauer": self.analyse_dauer,
            "einstellungen": self.einstellungen,
            "profil": {
                "name": profil.name,
                "verifiziert": profil.verifiziert,
                "hinweis": profil.hinweis,
                "seiten_min": profil.seiten_min,
                "seiten_schritt": profil.seiten_schritt,
                "seiten_max": profil.seiten_max,
                "auswahl": PROFILE,
            },
            "zahlen": {
                "dateien": self.fotos.len(),
                "lesbar": lesbar,
                "nicht_lesbar": self.fotos.len() - lesbar,
                "endungen": endungen,
                "ohne_aufnahmezeit": self.fotos.iter().filter(|f| f.fehler.is_empty() && f.zeit_quelle == "datei").count(),
                "ereignisse": erg.ereignisse.len(),
                "exakte_gruppen": erg.exakt.len(),
                "serien": erg.serien.len(),
                "serien_entschieden": serien_entschieden,
                "screenshots": erg.screenshots.len(),
                "im_buch": im_buch,
                "entfernt": self.fotos.len() - im_buch,
                "entfernt_gruende": entfernt_gruende,
                "eigene_entscheidungen": self.entscheidungen.len(),
            },
            "ereignisse": ereignisse,
            "orte": orte_ansicht,
            "zahlen_orte": {
                "orte": erg.orte.len(),
                "mit_gps": mit_gps,
                "ohne_ort": ohne_ort,
                "ort_entfernt": ort_entfernt,
            },
            "serien": serien,
            "fotos": fotos_kompakt,
            "seitenplan": seitenplan,
            "vorschlaege": {"auffuellen": idl(&auf), "kuerzen": idl(&kuerzen)},
            "buch": self.buch_seiten(&zahlen, &seitenplan),
        })
    }

    // ------------------------------------------------------------------ Vorschaubilder

    pub fn vorschau_ziel(&self, id: &str, kante: u32) -> Res<(PathBuf, PathBuf, String, u32)> {
        let kante = kante.clamp(64, 1600);
        let i = *self.id_index.get(id).ok_or("Unbekanntes Foto")?;
        let ziel = self.daten.join("vorschau").join(format!("{id}_{kante}.jpg"));
        Ok((ziel, self.ordner.join(&self.fotos[i].pfad), self.fotos[i].endung.clone(), kante))
    }

    // ------------------------------------------------------------------ Export

    /// Kopiert die Fotos im Buch nummeriert in Ereignis-Ordner und schreibt die Seitenliste.
    ///
    /// Originale bleiben unverändert. Ein bestehender, nicht leerer Zielordner wird nie überschrieben:
    /// stattdessen entsteht ein neuer Unterordner mit Zeitstempel.
    pub fn exportieren(&mut self, ziel: Option<&str>, bilder_kopieren: bool) -> Res<Value> {
        let mut ziel = match ziel.filter(|z| !z.trim().is_empty()) {
            Some(z) => PathBuf::from(z.trim()),
            None => self.ordner.join("fotobuch-export"),
        };
        if ziel.exists() && std::fs::read_dir(&ziel).map(|mut d| d.next().is_some()).unwrap_or(false) {
            ziel = ziel.join(format!("export-{}", Local::now().format("%Y%m%d-%H%M%S")));
        }
        std::fs::create_dir_all(&ziel).map_err(|e| format!("Zielordner kann nicht angelegt werden: {e}"))?;
        let einst = self.einstufungen();
        let zahlen = self.ereignis_zahlen(&einst);
        let plan = self.seitenplan_aus(&zahlen);
        let fps = self.einstellungen.fotos_pro_seite.max(1);
        let getrennt = self.einstellungen.ereignisse_getrennt;
        let erg = self.erg();

        let mut reihenfolge: Vec<(usize, usize, String)> = Vec::new(); // (Ereignis, Foto, Pfad im Export)
        let (mut nr, mut kopiert) = (0usize, 0usize);
        let mut fehler: Vec<String> = Vec::new();
        for (ei, (_ids, buch)) in zahlen.iter().enumerate() {
            if buch.is_empty() {
                continue;
            }
            let key = self.ids[erg.ereignisse[ei].fotos[0]].clone();
            let name = self.ereignis_namen.get(&key).cloned().unwrap_or_else(|| erg.ereignisse[ei].titel(&self.fotos));
            let ordnername = kuerze(&sauber(&format!("{:02}_{}", ei + 1, name)), 80);
            for &i in buch {
                nr += 1;
                let f = &self.fotos[i];
                let zielname = format!("{:04}_{}", nr, sauber(&f.name));
                let rel = format!("{ordnername}/{zielname}");
                if bilder_kopieren {
                    let _ = std::fs::create_dir_all(ziel.join(&ordnername));
                    match std::fs::copy(self.ordner.join(&f.pfad), ziel.join(&ordnername).join(&zielname)) {
                        Ok(_) => kopiert += 1,
                        Err(e) => fehler.push(format!("{}: {e}", f.name)),
                    }
                }
                reihenfolge.push((ei, i, rel));
            }
        }

        let counts: Vec<i64> = zahlen.iter().map(|(_, b)| b.len() as i64).collect();
        let seiten = seitenzuordnung(&counts, fps, getrennt);
        let mut zeilen: Vec<Vec<String>> = Vec::new();
        let mut pos = 0usize;
        for (seitennr, (_ei, n)) in seiten.iter().enumerate() {
            for _ in 0..*n {
                let (ei, i, rel) = &reihenfolge[pos];
                let f = &self.fotos[*i];
                let key = self.ids[erg.ereignisse[*ei].fotos[0]].clone();
                let name = self.ereignis_namen.get(&key).cloned().unwrap_or_else(|| erg.ereignisse[*ei].titel(&self.fotos));
                let ort = erg.ort_von[*i].map(|o| self.ort_name(o)).unwrap_or_default();
                zeilen.push(vec![(seitennr + 1).to_string(), name, rel.clone(), f.name.clone(), f.zeit.format("%Y-%m-%d %H:%M").to_string(), ort]);
                pos += 1;
            }
        }
        let buch_seiten = plan["buch_seiten"].as_i64().unwrap_or(0);
        for leer in (seiten.len() as i64 + 1)..=buch_seiten {
            zeilen.push(vec![leer.to_string(), "(frei)".into(), "".into(), "".into(), "".into(), "".into()]);
        }
        let mut csv = String::from("\u{feff}Seite;Ereignis;Datei im Export;Originalname;Aufnahmezeit;Ort\r\n");
        for z in &zeilen {
            let zellen: Vec<String> = z
                .iter()
                .map(|c| if c.contains(';') || c.contains('"') || c.contains('\n') { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.clone() })
                .collect();
            csv.push_str(&zellen.join(";"));
            csv.push_str("\r\n");
        }
        let _ = std::fs::write(ziel.join("seitenliste.csv"), csv);
        let liesmich = format!(
            "Fotobuch-Assistent: Export\nErstellt am {}\n\nFotos im Buch: {}\nSeiten (benötigt / Buch): {} / {}\nFotos pro Seite: {}, Ereignisse beginnen auf neuer Seite: {}\n\nSo geht es weiter:\n1. Öffne die Software deines Druckdienstes und lade die Fotos aus den nummerierten Ordnern hoch.\n   Die Nummern am Dateianfang geben die Reihenfolge vor.\n2. In seitenliste.csv steht, welches Foto auf welche Seite gehört.\n3. Die Originalfotos wurden nicht verändert.\n",
            Local::now().format("%d.%m.%Y %H:%M"),
            nr,
            plan["seiten_benoetigt"],
            buch_seiten,
            fps,
            if getrennt { "ja" } else { "nein" }
        );
        let _ = std::fs::write(ziel.join("LIESMICH.txt"), liesmich);
        Ok(json!({"ziel": ziel.to_string_lossy(), "fotos": nr, "kopiert": kopiert, "seiten": buch_seiten, "fehler": fehler}))
    }
}

// ---------------------------------------------------------------------------- Analyse-Lauf

pub type Fortschritt<'a> = &'a (dyn Fn(&str, usize, usize) + Sync);

/// Liest neue und geänderte Fotos ein. Unveränderte kommen aus dem Zwischenspeicher. Das Projekt ist nur
/// kurz gesperrt, damit die Oberfläche währenddessen bedienbar bleibt.
pub fn analysieren(projekt: &Arc<Mutex<Projekt>>, fortschritt: Fortschritt) -> Res<()> {
    let t0 = Instant::now();
    let (ordner, alt): (PathBuf, HashMap<String, (u64, u64)>) = {
        let p = projekt.lock().map_err(|_| "Interner Fehler")?;
        (p.ordner.clone(), p.cache.iter().map(|(k, e)| (k.clone(), (e.mtime_ns, e.groesse))).collect())
    };
    fortschritt("suchen", 0, 0);
    let pfade = finde_fotos(&ordner);
    let mut vorhanden: Vec<(String, PathBuf, u64, u64, bool)> = Vec::new();
    for p in pfade {
        let rel = p.strip_prefix(&ordner).unwrap_or(&p).to_string_lossy().replace('\\', "/");
        let (size, ns) = datei_zeiten(&p);
        let ns = ns.min(u64::MAX as u128) as u64;
        let unveraendert = alt.get(&rel).map_or(false, |&(m, g)| m == ns && g == size);
        vorhanden.push((rel, p, ns, size, unveraendert));
    }
    let zu_lesen: Vec<&(String, PathBuf, u64, u64, bool)> = vorhanden.iter().filter(|v| !v.4).collect();
    let n = zu_lesen.len();
    fortschritt("einlesen", 0, n);
    let fertig = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(2, 16);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().map_err(|e| e.to_string())?;
    let neue: Vec<(String, CacheEintrag)> = pool.install(|| {
        zu_lesen
            .par_iter()
            .map(|(rel, p, ns, size, _)| {
                let foto = analysiere_foto(p, rel);
                let k = fertig.fetch_add(1, Ordering::SeqCst) + 1;
                fortschritt("einlesen", k, n);
                (rel.clone(), CacheEintrag { mtime_ns: *ns, groesse: *size, foto })
            })
            .collect()
    });

    let mut p = projekt.lock().map_err(|_| "Interner Fehler")?;
    let mut neuer_cache: BTreeMap<String, CacheEintrag> = BTreeMap::new();
    for (rel, _, _, _, unveraendert) in &vorhanden {
        if *unveraendert {
            if let Some(e) = p.cache.get(rel) {
                neuer_cache.insert(rel.clone(), e.clone());
            }
        }
    }
    for (rel, e) in neue {
        neuer_cache.insert(rel, e);
    }
    p.cache = neuer_cache;
    p.fotos_aus_cache();
    p.analysiert_am = Some(Local::now().format("%Y-%m-%dT%H:%M:%S").to_string());
    p.analyse_dauer = Some((t0.elapsed().as_secs_f64() * 100.0).round() / 100.0);
    // Entscheidungen zu verschwundenen Fotos verwerfen
    let ids: HashSet<String> = p.ids.iter().cloned().collect();
    p.entscheidungen.retain(|k, _| ids.contains(k));
    if p.fotos.is_empty() {
        p.ergebnis = None;
    } else {
        p.neu_gruppieren();
    }
    p.speichern();
    Ok(())
}

// ---------------------------------------------------------------------------- Vorschau erzeugen

/// Erzeugt (oder liest aus dem Zwischenspeicher) ein JPEG-Vorschaubild. Läuft ohne Projektsperre.
pub fn vorschau_bytes(ziel: &Path, quelle: &Path, endung: &str, kante: u32) -> Vec<u8> {
    if let Ok(b) = std::fs::read(ziel) {
        return b;
    }
    let bytes = (|| -> Result<Vec<u8>, String> {
        let daten = std::fs::read(quelle).map_err(|e| e.to_string())?;
        let g = scanner::lade_bild(quelle, &daten, endung, kante)?;
        let orient = scanner::orientierung_von(&g);
        let img = scanner::passe_ein(&scanner::drehe(g.rgb, orient), kante, FilterType::CatmullRom);
        let mut out = Vec::new();
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82);
        img.write_with_encoder(enc).map_err(|e| e.to_string())?;
        Ok(out)
    })()
    .unwrap_or_else(|_| platzhalter(kante));
    if let Some(dir) = ziel.parent() {
        if std::fs::create_dir_all(dir).is_ok() {
            let _ = std::fs::write(ziel, &bytes);
        }
    }
    bytes
}

fn platzhalter(kante: u32) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(kante, (kante as f64 * 0.75) as u32, image::Rgb([200, 205, 210]));
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 70);
    let _ = img.write_with_encoder(enc);
    out
}
