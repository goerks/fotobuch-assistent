//! Projektzustand der App: Fotos, Gruppen, Entscheidungen, Seitenplan, Speicherung, Export.
//!
//! Ein Projekt gehört zu einem Fotoordner. Alles wird im Unterordner `.fotobuch` des Fotoordners
//! abgelegt (Zustand und Vorschaubilder). Die Originalfotos werden nie verändert.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{Datelike, Local};
use image::imageops::FilterType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};

use crate::duplicates::{finde_aehnliche, finde_exakte, Gruppe, SerienOptionen};
use crate::events::{bilde_ereignisse, Ereignis};
use crate::foto::Foto;
use crate::gestaltung::{feld_im_bereich, monat_name, Gestaltung};
use crate::layout::layout_varianten;
use crate::orte::{bilde_orte, Ort};
use crate::pages::{lade_profil, Profil, PROFILE};
use crate::umfang::{self, Eingabe};
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
    /// Bereich «Fotos pro Seite»: Jedes Ereignis bekommt so viele Seiten, dass es hineinpasst.
    pub pro_seite_von: i64,
    pub pro_seite_bis: i64,
    /// Gewünschte Seitenzahl des Buchs; leer = so viele wie nötig.
    pub seiten_ziel: Option<i64>,
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
            pro_seite_von: 2,
            pro_seite_bis: 5,
            seiten_ziel: None,
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
    layout_wahl: BTreeMap<String, usize>,
    seitenart: BTreeMap<String, String>,
    verschoben: BTreeMap<String, String>,
    ereignis_teilen: BTreeSet<String>,
    ereignis_verbinden: BTreeSet<String>,
    ereignis_dichte: BTreeMap<String, i64>,
    ereignis_seiten: BTreeMap<String, i64>,
    gestaltung: Gestaltung,
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

/// Ab diesem Seitenverhältnis (Breite : Höhe) bekommt ein Foto automatisch eine eigene Seite.
pub const PANORAMA_AB: f64 = 2.2;
/// Ab diesem Seitenverhältnis läuft es automatisch über eine ganze Doppelseite.
pub const DOPPEL_AB: f64 = 2.8;
const ART_NAMEN: [&str; 3] = ["normal", "seite", "doppel"];

pub struct Projekt {
    pub ordner: PathBuf,
    pub daten: PathBuf,
    pub einstellungen: Einstellungen,
    pub entscheidungen: BTreeMap<String, String>,
    pub ereignis_namen: BTreeMap<String, String>,
    pub ort_namen: BTreeMap<String, String>,
    pub layout_wahl: BTreeMap<String, usize>,
    pub seitenart: BTreeMap<String, String>,
    pub verschoben: BTreeMap<String, String>,
    pub ereignis_teilen: BTreeSet<String>,
    pub ereignis_verbinden: BTreeSet<String>,
    pub ereignis_dichte: BTreeMap<String, i64>,
    pub ereignis_seiten: BTreeMap<String, i64>,
    pub gestaltung: Gestaltung,
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
            layout_wahl: BTreeMap::new(),
            seitenart: BTreeMap::new(),
            verschoben: BTreeMap::new(),
            ereignis_teilen: BTreeSet::new(),
            ereignis_verbinden: BTreeSet::new(),
            ereignis_dichte: BTreeMap::new(),
            ereignis_seiten: BTreeMap::new(),
            gestaltung: Gestaltung::default(),
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
                self.layout_wahl = z.layout_wahl;
                self.seitenart = z.seitenart;
                self.verschoben = z.verschoben;
                self.ereignis_teilen = z.ereignis_teilen;
                self.ereignis_verbinden = z.ereignis_verbinden;
                self.ereignis_dichte = z.ereignis_dichte;
                self.ereignis_seiten = z.ereignis_seiten;
                self.gestaltung = z.gestaltung;
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
            layout_wahl: self.layout_wahl.clone(),
            seitenart: self.seitenart.clone(),
            verschoben: self.verschoben.clone(),
            ereignis_teilen: self.ereignis_teilen.clone(),
            ereignis_verbinden: self.ereignis_verbinden.clone(),
            ereignis_dichte: self.ereignis_dichte.clone(),
            ereignis_seiten: self.ereignis_seiten.clone(),
            gestaltung: self.gestaltung.clone(),
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
        let ereignisse = Self::ereignisse_anpassen(bilde_ereignisse(fotos, e.luecke_stunden, e.max_km), &self.ids, &self.ereignis_teilen, &self.ereignis_verbinden);
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

    /// Wendet die manuellen Änderungen an: erst Teilungen, dann Zusammenlegungen.
    fn ereignisse_anpassen(auto: Vec<Ereignis>, ids: &[String], teilen: &BTreeSet<String>, verbinden: &BTreeSet<String>) -> Vec<Ereignis> {
        let mut gruppen: Vec<Vec<usize>> = Vec::new();
        for ev in auto {
            let mut aktuell: Vec<usize> = Vec::new();
            for i in ev.fotos {
                if !aktuell.is_empty() && teilen.contains(&ids[i]) {
                    gruppen.push(std::mem::take(&mut aktuell));
                }
                aktuell.push(i);
            }
            if !aktuell.is_empty() {
                gruppen.push(aktuell);
            }
        }
        let mut aus: Vec<Vec<usize>> = Vec::new();
        for g in gruppen {
            if !aus.is_empty() && verbinden.contains(&ids[g[0]]) {
                aus.last_mut().unwrap().extend(g);
            } else {
                aus.push(g);
            }
        }
        aus.into_iter().enumerate().map(|(k, fotos)| Ereignis { nummer: k + 1, fotos }).collect()
    }

    /// Ereignis ab diesem Foto neu beginnen lassen (oder eine frühere Zusammenlegung an dieser Stelle aufheben).
    pub fn ereignis_teilen_bei(&mut self, id: &str) -> Res<()> {
        let i = *self.id_index.get(id).ok_or("Unbekanntes Foto")?;
        let ev = &self.erg().ereignisse[self.erg().ereignis_von[i]];
        if ev.fotos[0] == i {
            return Err("Dieses Foto beginnt schon ein Ereignis.".into());
        }
        if !self.ereignis_verbinden.remove(id) {
            self.ereignis_teilen.insert(id.to_string());
        }
        self.neu_gruppieren();
        self.speichern();
        Ok(())
    }

    /// Ereignis mit dem davor liegenden zusammenlegen (oder eine frühere Teilung aufheben).
    pub fn ereignis_mit_vorherigem(&mut self, key: &str) -> Res<()> {
        let i = *self.id_index.get(key).ok_or("Unbekanntes Ereignis")?;
        let ev = &self.erg().ereignisse[self.erg().ereignis_von[i]];
        if ev.fotos[0] != i {
            return Err("Unbekanntes Ereignis".into());
        }
        if ev.nummer == 1 {
            return Err("Das erste Ereignis hat kein vorheriges.".into());
        }
        if !self.ereignis_teilen.remove(key) {
            self.ereignis_verbinden.insert(key.to_string());
        }
        self.neu_gruppieren();
        self.speichern();
        Ok(())
    }

    pub fn ereignisse_zuruecksetzen(&mut self) {
        self.ereignis_teilen.clear();
        self.ereignis_verbinden.clear();
        self.neu_gruppieren();
        self.speichern();
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
                "aehnlich_abstand" | "ort_radius_m" | "pro_seite_von" | "pro_seite_bis" => {
                    let (lo, hi) = match k.as_str() {
                        "aehnlich_abstand" => (0, 16),
                        "ort_radius_m" => (25, 5000),
                        _ => (1, 12),
                    };
                    let x = ganzzahl_aus(v).ok_or_else(|| format!("Ungültiger Wert für {k}"))?;
                    if !(lo..=hi).contains(&x) {
                        return Err(format!("{k} muss zwischen {lo} und {hi} liegen"));
                    }
                    match k.as_str() {
                        "aehnlich_abstand" => e.aehnlich_abstand = x,
                        "ort_radius_m" => e.ort_radius_m = x,
                        "pro_seite_von" => {
                            e.pro_seite_von = x;
                            e.pro_seite_bis = e.pro_seite_bis.max(x);
                        }
                        _ => {
                            e.pro_seite_bis = x;
                            e.pro_seite_von = e.pro_seite_von.min(x);
                        }
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
                "seiten_min" | "seiten_schritt" | "seiten_max" | "max_pro_ort" | "seiten_ziel" => {
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
                        "seiten_ziel" => e.seiten_ziel = x,
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

    /// Wählt für eine Seite (Schlüssel = erstes Foto) eines der angebotenen Layouts.
    pub fn setze_layout(&mut self, key: &str, wahl: usize) -> Res<()> {
        if !self.id_index.contains_key(key) {
            return Err("Unbekannte Seite".into());
        }
        if wahl == 0 {
            self.layout_wahl.remove(key);
        } else {
            self.layout_wahl.insert(key.to_string(), wahl.min(64));
        }
        self.speichern();
        Ok(())
    }

    pub fn setze_schritt(&mut self, schritt: i64) {
        self.schritt = schritt.clamp(1, 7);
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

    /// Eingabe der Seitenrechnung je Ereignis (oder eine Gruppe, wenn Ereignisse nicht getrennt werden).
    fn umfang_eingabe(&self, zahlen: &[(Vec<usize>, Vec<usize>)]) -> Vec<Eingabe> {
        let erg = self.erg();
        let je_ereignis: Vec<Eingabe> = zahlen
            .iter()
            .enumerate()
            .map(|(ei, (_, b))| {
                let key = &self.ids[erg.ereignisse[ei].fotos[0]];
                let (mut normal, mut sonder) = (0, 0);
                for &i in b {
                    match self.seitenart(i) {
                        0 => normal += 1,
                        1 => sonder += 1,
                        _ => sonder += 2,
                    }
                }
                Eingabe { normal, sonder, dichte: self.ereignis_dichte.get(key).copied().unwrap_or(0), manuell: self.ereignis_seiten.get(key).copied() }
            })
            .collect();
        if self.einstellungen.ereignisse_getrennt {
            je_ereignis
        } else {
            vec![Eingabe { normal: je_ereignis.iter().map(|e| e.normal).sum(), sonder: je_ereignis.iter().map(|e| e.sonder).sum(), ..Default::default() }]
        }
    }

    pub fn umfang_plan(&self, zahlen: &[(Vec<usize>, Vec<usize>)]) -> umfang::Plan {
        let e = &self.einstellungen;
        umfang::plane(&self.umfang_eingabe(zahlen), e.pro_seite_von, e.pro_seite_bis, &self.profil(), e.seiten_ziel)
    }

    pub fn seitenverhaeltnis_foto(&self, i: usize) -> f64 {
        self.fotos[i].breite.max(1) as f64 / self.fotos[i].hoehe.max(1) as f64
    }

    /// Seitenart eines Fotos: 0 normal, 1 eigene Seite, 2 über die Doppelseite. Panoramen automatisch, sonst nach Wahl.
    pub fn seitenart(&self, i: usize) -> u8 {
        match self.seitenart.get(&self.ids[i]).map(|s| s.as_str()) {
            Some("normal") => 0,
            Some("seite") => 1,
            Some("doppel") => 2,
            _ => {
                let ar = self.seitenverhaeltnis_foto(i);
                if ar >= DOPPEL_AB {
                    2
                } else if ar >= PANORAMA_AB {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// `art`: «normal», «seite» oder «doppel»; None stellt die automatische Wahl wieder her.
    pub fn setze_seitenart(&mut self, id: &str, art: Option<&str>) -> Res<()> {
        if !self.id_index.contains_key(id) {
            return Err("Unbekanntes Foto".into());
        }
        match art {
            Some(a @ ("normal" | "seite" | "doppel")) => {
                self.seitenart.insert(id.to_string(), a.to_string());
                if a != "normal" {
                    self.verschoben.remove(id);
                }
            }
            None => {
                self.seitenart.remove(id);
            }
            _ => return Err("Unbekannte Seitenart".into()),
        }
        self.speichern();
        Ok(())
    }

    /// Foto auf eine andere Seite des Buchs legen (`seite` = Seitennummer) oder die Verschiebung aufheben (None).
    pub fn verschiebe_foto(&mut self, id: &str, seite: Option<usize>) -> Res<()> {
        let xi = *self.id_index.get(id).ok_or("Unbekanntes Foto")?;
        match seite {
            None => {
                self.verschoben.remove(id);
            }
            Some(nr) => {
                if self.seitenart(xi) != 0 {
                    return Err("Dieses Foto hat eine eigene Seite. Stelle es zuerst auf «normal».".into());
                }
                let einst = self.einstufungen();
                let zahlen = self.ereignis_zahlen(&einst);
                let seiten = self.seiten_gruppen_ex(&zahlen);
                let ziel = seiten.get(nr.wrapping_sub(1)).ok_or("Diese Seite gibt es nicht.")?;
                if ziel.2 != 0 {
                    return Err("Auf dieser Seite steht ein Foto über die ganze Seite.".into());
                }
                if ziel.1.contains(&xi) {
                    return Ok(());
                }
                let anker = ziel
                    .1
                    .iter()
                    .copied()
                    .find(|&k| k != xi && !self.verschoben.contains_key(&self.ids[k]))
                    .ok_or("Auf dieser Seite fehlt ein fest platziertes Foto als Ziel.")?;
                self.verschoben.insert(id.to_string(), self.ids[anker].clone());
            }
        }
        self.speichern();
        Ok(())
    }

    pub fn verschiebungen_zuruecksetzen(&mut self) {
        self.verschoben.clear();
        self.speichern();
    }

    /// Seitenzähler für die App: Seiten je Ereignis, Buchgrösse und Spielraum.
    pub fn seitenplan_aus(&self, zahlen: &[(Vec<usize>, Vec<usize>)]) -> Value {
        let plan = self.umfang_plan(zahlen);
        let e = &self.einstellungen;
        let profil = self.profil();
        let getrennt = e.ereignisse_getrennt;
        let total: usize = zahlen.iter().map(|(_, b)| b.len()).sum();
        let erg = self.erg();
        // Je Ereignis: bei getrennten Ereignissen der eigene Plan, sonst anteilig
        let normal_gesamt: i64 = plan.gruppen.iter().map(|g| g.normal_seiten).sum::<i64>().max(1);
        let ereignisse: Vec<Value> = zahlen
            .iter()
            .enumerate()
            .map(|(ei, (_, b))| {
                let key = self.ids[erg.ereignisse[ei].fotos[0]].clone();
                let dichte = self.ereignis_dichte.get(&key).copied().unwrap_or(0);
                if getrennt {
                    let g = &plan.gruppen[ei];
                    json!({"seiten": g.seiten(), "normal_seiten": g.normal_seiten, "min": g.min, "max": g.max, "start": g.start, "links": g.beginnt_links() && !b.is_empty(),
                        "manuell": g.manuell, "dichte": dichte, "fotos": b.len(), "pro_seite": if g.normal_seiten > 0 { (b.len() as f64 / g.seiten().max(1) as f64 * 10.0).round() / 10.0 } else { 0.0 }})
                } else {
                    let anteil = if total == 0 { 0 } else { ((b.len() as f64 / total as f64) * normal_gesamt as f64).round() as i64 };
                    json!({"seiten": anteil, "normal_seiten": anteil, "min": 0, "max": 0, "start": 0, "links": false, "manuell": false, "dichte": 0, "fotos": b.len(), "pro_seite": 0.0})
                }
            })
            .collect();
        json!({
            "fotos": total,
            "pro_seite_von": e.pro_seite_von,
            "pro_seite_bis": e.pro_seite_bis,
            "ziel": e.seiten_ziel,
            "ereignisse_getrennt": getrennt,
            "seiten_benoetigt": plan.summe,
            "buch_seiten": plan.buch,
            "leere_seiten": plan.leer,
            "ueber_maximum": plan.buch > profil.max(),
            "kleinere_stufe": plan.kleinere,
            "groessere_stufe": plan.groessere,
            "seiten_je_ereignis": ereignisse.iter().map(|e| e["seiten"].clone()).collect::<Vec<_>>(),
            "ereignisse": ereignisse,
        })
    }

    pub fn seitenplan(&self) -> Value {
        let einst = self.einstufungen();
        self.seitenplan_aus(&self.ereignis_zahlen(&einst))
    }

    /// Dichte (-1 kompakt, 0 normal, 1 gross) und/oder feste Seitenzahl eines Ereignisses. `None` setzt zurück.
    pub fn ereignis_umfang(&mut self, key: &str, dichte: Option<Option<i64>>, seiten: Option<Option<i64>>) -> Res<()> {
        if !self.id_index.contains_key(key) {
            return Err("Unbekanntes Ereignis".into());
        }
        if let Some(d) = dichte {
            match d {
                Some(x) if (-1..=1).contains(&x) && x != 0 => {
                    self.ereignis_dichte.insert(key.to_string(), x);
                }
                _ => {
                    self.ereignis_dichte.remove(key);
                }
            }
        }
        if let Some(s) = seiten {
            match s {
                Some(x) if (1..=500).contains(&x) => {
                    self.ereignis_seiten.insert(key.to_string(), x);
                }
                None => {
                    self.ereignis_seiten.remove(key);
                }
                _ => return Err("Ungültige Seitenzahl".into()),
            }
        }
        self.speichern();
        Ok(())
    }

    pub fn umfang_zuruecksetzen(&mut self) {
        self.ereignis_dichte.clear();
        self.ereignis_seiten.clear();
        self.speichern();
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

    /// Verteilt die Fotos im Buch auf Seiten: (Ereignis, Fotos der Seite). Fotos mit eigener Ganzseite bekommen
    /// je eine Seite (nach der gerade gefüllten Seite), alle anderen werden gleichmässig verteilt und danach
    /// an den Seitengrenzen so getauscht, dass die Seiten möglichst gut gefüllt sind (Hoch- und Querformate passen zusammen).
    fn seiten_gruppen(&self, zahlen: &[(Vec<usize>, Vec<usize>)]) -> Vec<(usize, Vec<usize>)> {
        self.seiten_gruppen_ex(zahlen).into_iter().map(|(e, f, _)| (e, f)).collect()
    }

    /// Wie `seiten_gruppen`, dazu die Seitenart: 0 normal, 1 ganze Seite, 2 linke und 3 rechte Hälfte eines Fotos über die Doppelseite.
    fn seiten_gruppen_ex(&self, zahlen: &[(Vec<usize>, Vec<usize>)]) -> Vec<(usize, Vec<usize>, u8)> {
        let getrennt = self.einstellungen.ereignisse_getrennt;
        let erg = self.erg();
        let plan = self.umfang_plan(zahlen);
        let schlangen: Vec<(usize, Vec<usize>)> = if getrennt {
            zahlen.iter().enumerate().map(|(ei, (_, b))| (ei, b.clone())).collect()
        } else {
            vec![(usize::MAX, zahlen.iter().flat_map(|(_, b)| b.iter().copied()).collect())]
        };
        let mut aus: Vec<(usize, Vec<usize>, u8)> = Vec::new();
        let sonder = |aus: &mut Vec<(usize, Vec<usize>, u8)>, ev: usize, i: usize| {
            if self.seitenart(i) == 1 {
                aus.push((ev, vec![i], 1));
            } else {
                aus.push((ev, vec![i], 2));
                aus.push((ev, vec![i], 3));
            }
        };
        for (ei, q) in schlangen {
            let normal = q.iter().filter(|&&i| self.seitenart(i) == 0).count() as i64;
            let gruppe = if getrennt { ei } else { 0 };
            let groessen = umfang::verteile_auf(normal, plan.gruppen.get(gruppe).map_or(1, |g| g.normal_seiten.max(1)));
            let (mut gi, mut puffer, mut wartend): (usize, Vec<usize>, Vec<usize>) = (0, Vec::new(), Vec::new());
            let ev_von = |i: usize| if ei == usize::MAX { erg.ereignis_von[i] } else { ei };
            for &i in &q {
                if self.seitenart(i) != 0 {
                    if puffer.is_empty() {
                        sonder(&mut aus, ev_von(i), i);
                    } else {
                        wartend.push(i);
                    }
                    continue;
                }
                puffer.push(i);
                if puffer.len() as i64 == groessen[gi] {
                    aus.push((ev_von(puffer[0]), std::mem::take(&mut puffer), 0));
                    gi += 1;
                    for w in wartend.drain(..) {
                        sonder(&mut aus, ev_von(w), w);
                    }
                }
            }
        }
        self.paarung_verbessern(&mut aus);
        self.verschiebungen_anwenden(&mut aus);
        // Ein Foto über die Doppelseite beginnt auf einer linken Seite (gerade Seitennummer, Seite 1 steht allein).
        let mut k = 0;
        while k < aus.len() {
            if aus[k].2 == 2 && k % 2 == 0 {
                if k + 2 < aus.len() && aus[k + 2].2 <= 1 {
                    let p = aus.remove(k + 2);
                    aus.insert(k, p);
                } else {
                    aus[k].2 = 1; // keine passende Nachbarseite: nur eine ganze Seite
                    aus.remove(k + 1);
                }
            }
            k += 1;
        }
        aus
    }

    /// Von Hand verschobene Fotos auf die gewählte Seite legen. Leere Seiten fallen weg.
    fn verschiebungen_anwenden(&self, seiten: &mut Vec<(usize, Vec<usize>, u8)>) {
        for (x, y) in &self.verschoben {
            let (Some(&xi), Some(&yi)) = (self.id_index.get(x), self.id_index.get(y)) else { continue };
            let von = seiten.iter().position(|s| s.2 == 0 && s.1.contains(&xi));
            let nach = seiten.iter().position(|s| s.2 == 0 && s.1.contains(&yi));
            if let (Some(a), Some(b)) = (von, nach) {
                if a != b {
                    seiten[a].1.retain(|&k| k != xi);
                    seiten[b].1.push(xi);
                    seiten[b].1.sort_by(|&p, &q| self.fotos[p].schluessel().cmp(&self.fotos[q].schluessel()));
                }
            }
        }
        seiten.retain(|s| !s.1.is_empty());
    }

    /// Tauscht Fotos an Seitengrenzen (jeweils eines der letzten zwei gegen eines der ersten zwei), wenn beide Seiten
    /// dadurch deutlich besser gefüllt werden. Die Reihenfolge weicht höchstens um eine Seitengrenze von der Zeit ab.
    fn paarung_verbessern(&self, seiten: &mut [(usize, Vec<usize>, u8)]) {
        let seite_ar = self.seiten_verhaeltnis();
        let ars = |l: &[usize]| -> Vec<f64> { l.iter().map(|&i| self.seitenverhaeltnis_foto(i)).collect() };
        let guete = |l: &[usize]| crate::layout::layout_guete(&ars(l), seite_ar);
        for _runde in 0..2 {
            let mut geaendert = false;
            for k in 0..seiten.len().saturating_sub(1) {
                let (links, rechts) = seiten.split_at_mut(k + 1);
                let (a, b) = (&mut links[k], &mut rechts[0]);
                let klein = |l: &Vec<usize>| l.len() >= 2 && l.len() <= 5;
                if a.2 != 0 || b.2 != 0 || a.0 != b.0 || !klein(&a.1) || !klein(&b.1) {
                    continue;
                }
                let basis = guete(&a.1) + guete(&b.1);
                let mut beste: Option<(f64, usize, usize)> = None;
                for x in a.1.len().saturating_sub(2)..a.1.len() {
                    for y in 0..2.min(b.1.len()) {
                        let (mut na, mut nb) = (a.1.clone(), b.1.clone());
                        std::mem::swap(&mut na[x], &mut nb[y]);
                        let gewinn = guete(&na) + guete(&nb) - basis;
                        if gewinn > 3.0 && beste.map_or(true, |(g, _, _)| gewinn > g) {
                            beste = Some((gewinn, x, y));
                        }
                    }
                }
                if let Some((_, x, y)) = beste {
                    std::mem::swap(&mut a.1[x], &mut b.1[y]);
                    let nach_zeit = |l: &mut Vec<usize>| l.sort_by(|&p, &q| self.fotos[p].schluessel().cmp(&self.fotos[q].schluessel()));
                    nach_zeit(&mut a.1);
                    nach_zeit(&mut b.1);
                    geaendert = true;
                }
            }
            if !geaendert {
                break;
            }
        }
    }

    /// Layouts eines Fotos über die ganze Doppelseite, in Prozent der Doppelseite.
    fn doppel_varianten(&self, foto: usize) -> Vec<Vec<[f64; 4]>> {
        layout_varianten(&[self.seitenverhaeltnis_foto(foto)], self.seiten_verhaeltnis() * 2.0, 8)
    }

    /// Layouts einer Seite in Prozent dieser Seite. `art` 2 und 3: linke und rechte Hälfte eines Doppelseiten-Fotos.
    fn seiten_varianten(&self, fotos: &[usize], art: u8) -> Vec<Vec<[f64; 4]>> {
        if art >= 2 {
            let versatz = if art == 2 { 0.0 } else { 50.0 };
            return self.doppel_varianten(fotos[0]).into_iter().map(|v| v.iter().map(|f| [(f[0] - versatz) * 2.0, f[1], f[2] * 2.0, f[3]]).collect()).collect();
        }
        let ars: Vec<f64> = fotos.iter().map(|&i| self.fotos[i].breite.max(1) as f64 / self.fotos[i].hoehe.max(1) as f64).collect();
        layout_varianten(&ars, self.seiten_verhaeltnis(), 8)
    }

    /// Alle angebotenen Layouts einer Seite (für die Vorlagenauswahl). Schlüssel = erstes Foto der Seite.
    pub fn layouts_fuer_seite(&self, key: &str) -> Res<Value> {
        let einst = self.einstufungen();
        let zahlen = self.ereignis_zahlen(&einst);
        let (_, fotos, art) = self
            .seiten_gruppen_ex(&zahlen)
            .into_iter()
            .find(|(_, f, _)| self.ids[f[0]] == key)
            .ok_or("Unbekannte Seite")?;
        // Ein Foto über die Doppelseite wird als ganze Doppelseite gezeigt
        let (varianten, ar) = if art >= 2 { (self.doppel_varianten(fotos[0]), self.seiten_verhaeltnis() * 2.0) } else { (self.seiten_varianten(&fotos, art), self.seiten_verhaeltnis()) };
        let wahl = self.layout_wahl.get(key).copied().unwrap_or(0) % varianten.len();
        let bereich = self.gestaltung.stil().bereich;
        let r = |x: f64| (x * 100.0).round() / 100.0;
        Ok(json!({
            "aktuell": wahl,
            "ar": ar,
            "fotos": fotos.iter().map(|&i| self.ids[i].clone()).collect::<Vec<_>>(),
            "layouts": varianten.iter().map(|v| v.iter().map(|f| { let f = feld_im_bereich(*f, bereich); json!([r(f[0]), r(f[1]), r(f[2]), r(f[3])]) }).collect::<Vec<_>>()).collect::<Vec<_>>(),
        }))
    }

    /// Seiten mit automatischem Layout. Die Fotos jedes Ereignisses folgen der Aufnahmezeit.
    fn buch_seiten(&self, zahlen: &[(Vec<usize>, Vec<usize>)], plan: &Value) -> Value {
        let ar_seite = self.seiten_verhaeltnis();
        let bereich = self.gestaltung.stil().bereich;
        let mut seiten = Vec::new();
        let mut vorher: Option<usize> = None;
        for (nr, (ereignis, fotos, ganz)) in self.seiten_gruppen_ex(zahlen).into_iter().enumerate() {
            let beschriftung = if vorher != Some(ereignis) { Some(self.ereignis_anzeigename(ereignis)) } else { None };
            vorher = Some(ereignis);
            let varianten = self.seiten_varianten(&fotos, ganz);
            let key = self.ids[fotos[0]].clone();
            let wahl = self.layout_wahl.get(&key).copied().unwrap_or(0) % varianten.len();
            let felder: Vec<[f64; 4]> = varianten[wahl].iter().map(|f| feld_im_bereich(*f, bereich)).collect();
            let r = |x: f64| (x * 100.0).round() / 100.0;
            seiten.push(json!({
                "beschriftung": beschriftung,
                "nr": nr + 1,
                "ereignis": ereignis,
                "titel": self.ereignis_anzeigename(ereignis),
                "key": key,
                "layout": wahl,
                "layouts": varianten.len(),
                "ganz": ganz > 0,
                "art": ganz,
                "fotos": fotos.iter().zip(felder.iter()).map(|(&i, f)| json!({"id": self.ids[i], "x": r(f[0]), "y": r(f[1]), "w": r(f[2]), "h": r(f[3])})).collect::<Vec<_>>(),
            }));
        }
        let zuordnung_len = seiten.len();
        let gesamt = plan["buch_seiten"].as_i64().unwrap_or(0) as usize;
        json!({
            "format": self.einstellungen.seitenformat,
            "ar": ar_seite,
            "seiten": seiten,
            "freie_seiten": (zuordnung_len + 1..=gesamt).collect::<Vec<_>>(),
        })
    }

    /// Alles, was zum Rendern der fertigen Seiten nötig ist (damit das Projekt dabei nicht gesperrt bleibt):
    /// Seiten, Deckblatt, Zahl der freien Seiten, Seitenformat und Stil.
    pub fn seitenauftraege(&self) -> Res<SeitenPaket> {
        if self.ergebnis.is_none() {
            return Err("Noch nicht analysiert".into());
        }
        let einst = self.einstufungen();
        let zahlen = self.ereignis_zahlen(&einst);
        let plan = self.seitenplan_aus(&zahlen);
        let stil = self.gestaltung.stil();
        let mut auftraege = Vec::new();
        let mut vorher: Option<usize> = None;
        for (nr, (ereignis, fotos, art)) in self.seiten_gruppen_ex(&zahlen).into_iter().enumerate() {
            let text = if vorher != Some(ereignis) { Some(self.ereignis_anzeigename(ereignis)) } else { None };
            vorher = Some(ereignis);
            let varianten = self.seiten_varianten(&fotos, art);
            let wahl = self.layout_wahl.get(&self.ids[fotos[0]]).copied().unwrap_or(0) % varianten.len();
            let felder = &varianten[wahl];
            auftraege.push(crate::seitenexport::Auftrag {
                nr: nr + 1,
                text,
                fotos: fotos
                    .iter()
                    .zip(felder.iter())
                    .map(|(&i, f)| crate::seitenexport::SeitenFoto { pfad: self.ordner.join(&self.fotos[i].pfad), endung: self.fotos[i].endung.clone(), feld: feld_im_bereich(*f, stil.bereich) })
                    .collect(),
            });
        }
        let gesamt = plan["buch_seiten"].as_i64().unwrap_or(0) as usize;
        let leere = gesamt.saturating_sub(auftraege.len());
        let titel_foto = self.titelfoto_index(&einst);
        let deckblatt = crate::seitenexport::DeckblattAuftrag {
            titel: if self.gestaltung.titel.trim().is_empty() { self.titel_auto() } else { self.gestaltung.titel.clone() },
            untertitel: if self.gestaltung.untertitel.trim().is_empty() { self.untertitel_auto(&einst) } else { self.gestaltung.untertitel.clone() },
            name: self.gestaltung.name.clone(),
            foto: titel_foto.map(|i| crate::seitenexport::SeitenFoto { pfad: self.ordner.join(&self.fotos[i].pfad), endung: self.fotos[i].endung.clone(), feld: [0.0, 0.0, 100.0, 100.0] }),
        };
        Ok(SeitenPaket { auftraege, deckblatt, leere, format: self.einstellungen.seitenformat.clone(), stil })
    }

    // ------------------------------------------------------------------ Gestaltung

    fn titel_auto(&self) -> String {
        let name = self.ordner.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        name.replace(['-', '_'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Zeitraum der Fotos im Buch als Text, zum Beispiel «14.–17. Mai 2026» (Fotos ohne Aufnahmezeit zählen nicht).
    fn untertitel_auto(&self, einst: &[Einstufung]) -> String {
        let zeiten: Vec<_> = (0..self.fotos.len()).filter(|&i| einst[i].behalten && self.fotos[i].zeit_quelle == "exif").map(|i| self.fotos[i].zeit.date()).collect();
        let (Some(von), Some(bis)) = (zeiten.iter().min().copied(), zeiten.iter().max().copied()) else { return String::new() };
        if von == bis {
            format!("{}. {} {}", von.day(), monat_name(von.month()), von.year())
        } else if (von.year(), von.month()) == (bis.year(), bis.month()) {
            format!("{}.–{}. {} {}", von.day(), bis.day(), monat_name(von.month()), von.year())
        } else if von.year() == bis.year() {
            format!("{}. {} – {}. {} {}", von.day(), monat_name(von.month()), bis.day(), monat_name(bis.month()), bis.year())
        } else {
            format!("{}. {} {} – {}. {} {}", von.day(), monat_name(von.month()), von.year(), bis.day(), monat_name(bis.month()), bis.year())
        }
    }

    /// Das gewählte Titelfoto, sonst das erste Foto im Buch.
    fn titelfoto_index(&self, einst: &[Einstufung]) -> Option<usize> {
        if let Some(&i) = self.id_index.get(&self.gestaltung.titelfoto) {
            if einst[i].behalten {
                return Some(i);
            }
        }
        let zahlen = self.ereignis_zahlen(einst);
        self.seiten_gruppen_ex(&zahlen).into_iter().next().map(|(_, f, _)| f[0])
    }

    pub fn setze_gestaltung(&mut self, neu: &serde_json::Map<String, Value>) -> Res<()> {
        if neu.get("zuruecksetzen").and_then(|v| v.as_bool()) == Some(true) {
            self.gestaltung.zurueck_auf_thema();
        }
        let mut rest = neu.clone();
        rest.remove("zuruecksetzen");
        if let Some(id) = rest.get("titelfoto").and_then(|v| v.as_str()) {
            if !id.is_empty() && !self.id_index.contains_key(id) {
                return Err("Unbekanntes Foto".into());
            }
        }
        self.gestaltung.setze(&rest)?;
        self.speichern();
        Ok(())
    }

    fn gestaltung_ansicht(&self, einst: &[Einstufung], zahlen: &[(Vec<usize>, Vec<usize>)]) -> Value {
        let titelfoto = self.titelfoto_index(einst).map(|i| self.ids[i].clone());
        let seiten = self.seiten_gruppen_ex(zahlen);
        let schritt = (seiten.len() / 8).max(1);
        let kandidaten: Vec<String> = seiten.iter().step_by(schritt).take(8).map(|(_, f, _)| self.ids[f[0]].clone()).collect();
        crate::gestaltung::ansicht(&self.gestaltung, &self.titel_auto(), &self.untertitel_auto(einst), titelfoto.as_deref(), &kandidaten)
    }

    /// Zielordner für die fertigen Seiten: `<Ziel>/seiten`, bei vorhandenem gefülltem Ordner mit Zeitstempel.
    pub fn seiten_zielordner(&self, ziel: Option<&str>) -> PathBuf {
        let basis = match ziel.filter(|z| !z.trim().is_empty()) {
            Some(z) => PathBuf::from(z.trim()),
            None => self.ordner.join("fotobuch-export"),
        };
        let mut d = basis.join("seiten");
        if d.exists() && std::fs::read_dir(&d).map(|mut x| x.next().is_some()).unwrap_or(false) {
            d = basis.join(format!("seiten-{}", Local::now().format("%Y%m%d-%H%M%S")));
        }
        d
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
        let mut seite_von: HashMap<usize, usize> = HashMap::new();
        for (nr, (_, fotos, art)) in self.seiten_gruppen_ex(&zahlen).into_iter().enumerate() {
            if art != 3 {
                for i in fotos {
                    seite_von.entry(i).or_insert(nr + 1);
                }
            }
        }
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
                "ids": idl(&ev.fotos),
                "ohne_datum": ev.ohne_datum(&self.fotos),
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
                    "art": ART_NAMEN[self.seitenart(i) as usize],
                    "pano": self.seitenverhaeltnis_foto(i) >= PANORAMA_AB,
                    "vs": self.verschoben.contains_key(&self.ids[i]),
                    "raw": f.ist_raw,
                    "o": erg.ort_von[i],
                    "q": (100.0 * score(f, self.maxs)).round() as i64,
                    "r": rang.get(&i),
                    "gd": self.begruendung(i, e, rang.get(&i).copied()),
                    "ort": erg.ort_von[i].map(|o| self.ort_name(o)),
                    "s": seite_von.get(&i),
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
            "gestaltung": self.gestaltung_ansicht(&einst, &zahlen),
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
                "verschiebungen": self.verschoben.len(),
                "ereignis_aenderungen": self.ereignis_teilen.len() + self.ereignis_verbinden.len(),
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

        let seiten = self.seiten_gruppen(&zahlen);
        let rel_von: HashMap<usize, (usize, String)> = reihenfolge.iter().map(|(ei, i, rel)| (*i, (*ei, rel.clone()))).collect();
        let mut zeilen: Vec<Vec<String>> = Vec::new();
        for (seitennr, (_ereignis, fotos)) in seiten.iter().enumerate() {
            for &i in fotos {
                let (ei, rel) = &rel_von[&i];
                let f = &self.fotos[i];
                let key = self.ids[erg.ereignisse[*ei].fotos[0]].clone();
                let name = self.ereignis_namen.get(&key).cloned().unwrap_or_else(|| erg.ereignisse[*ei].titel(&self.fotos));
                let ort = erg.ort_von[i].map(|o| self.ort_name(o)).unwrap_or_default();
                zeilen.push(vec![(seitennr + 1).to_string(), name, rel.clone(), f.name.clone(), f.zeit.format("%Y-%m-%d %H:%M").to_string(), ort]);
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
            "Fotobuch-Assistent: Export\nErstellt am {}\n\nFotos im Buch: {}\nSeiten (benötigt / Buch): {} / {}\nFotos pro Seite: {} bis {}, Ereignisse beginnen auf neuer Seite: {}\n\nSo geht es weiter:\n1. Öffne die Software deines Druckdienstes und lade die Fotos aus den nummerierten Ordnern hoch.\n   Die Nummern am Dateianfang geben die Reihenfolge vor.\n2. In seitenliste.csv steht, welches Foto auf welche Seite gehört.\n3. Die Originalfotos wurden nicht verändert.\n",
            Local::now().format("%d.%m.%Y %H:%M"),
            nr,
            plan["seiten_benoetigt"],
            buch_seiten,
            self.einstellungen.pro_seite_von,
            self.einstellungen.pro_seite_bis,
            if getrennt { "ja" } else { "nein" }
        );
        let _ = std::fs::write(ziel.join("LIESMICH.txt"), liesmich);
        Ok(json!({"ziel": ziel.to_string_lossy(), "fotos": nr, "kopiert": kopiert, "seiten": buch_seiten, "fehler": fehler}))
    }
}

// ---------------------------------------------------------------------------- Analyse-Lauf

/// Alles für den Export der fertigen Seiten.
pub struct SeitenPaket {
    pub auftraege: Vec<crate::seitenexport::Auftrag>,
    pub deckblatt: crate::seitenexport::DeckblattAuftrag,
    pub leere: usize,
    pub format: String,
    pub stil: crate::gestaltung::Stil,
}

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
