//! Seitenzähler: Wie viele Seiten ergeben die Fotos, und wie viele fehlen bis zur nächsten Seitenzahl?

use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize)]
pub struct Profil {
    pub name: String,
    #[serde(default)]
    pub verifiziert: bool,
    pub seiten_min: Option<i64>,
    pub seiten_schritt: Option<i64>,
    pub seiten_max: Option<i64>,
    #[serde(default)]
    pub hinweis: String,
}

pub const PROFILE: [&str; 3] = ["allgemein", "cewe-pixum", "ifolor"];

impl Profil {
    /// Fehlende Profilwerte durch Vorgaben oder Annahmen ersetzen.
    pub fn mit_vorgaben(&self, min: Option<i64>, schritt: Option<i64>, max: Option<i64>) -> Profil {
        Profil {
            name: self.name.clone(),
            verifiziert: self.verifiziert,
            seiten_min: Some(min.or(self.seiten_min).unwrap_or(24)),
            seiten_schritt: Some(schritt.or(self.seiten_schritt).unwrap_or(2)),
            seiten_max: Some(max.or(self.seiten_max).unwrap_or(200)),
            hinweis: self.hinweis.clone(),
        }
    }
    pub fn min(&self) -> i64 {
        self.seiten_min.unwrap_or(24)
    }
    pub fn schritt(&self) -> i64 {
        self.seiten_schritt.unwrap_or(2)
    }
    pub fn max(&self) -> i64 {
        self.seiten_max.unwrap_or(200)
    }
}

pub fn lade_profil(name: &str) -> Option<Profil> {
    let text = match name {
        "allgemein" => include_str!("../profiles/allgemein.json"),
        "cewe-pixum" => include_str!("../profiles/cewe-pixum.json"),
        "ifolor" => include_str!("../profiles/ifolor.json"),
        _ => return None,
    };
    serde_json::from_str(text).ok()
}

fn aufrunden(a: i64, b: i64) -> i64 {
    (a + b - 1).div_euclid(b)
}

/// Kleinste erlaubte Seitenzahl, die mindestens `benoetigt` Seiten bietet.
pub fn erlaubte_seitenzahl(benoetigt: i64, p: &Profil) -> i64 {
    let (mn, schritt) = (p.min(), p.schritt());
    if benoetigt <= mn {
        return mn;
    }
    mn + aufrunden(benoetigt - mn, schritt) * schritt
}

/// Verteilt `n` Fotos möglichst gleichmässig auf die nötigen Seiten (je höchstens `fps`).
pub fn verteile(n: i64, fps: i64) -> Vec<i64> {
    if n <= 0 {
        return vec![];
    }
    let seiten = aufrunden(n, fps);
    let (basis, rest) = (n / seiten, n % seiten);
    (0..seiten).map(|i| if i < rest { basis + 1 } else { basis }).collect()
}

fn gruppen(counts: &[i64], getrennt: bool) -> Vec<i64> {
    if getrennt {
        counts.iter().copied().filter(|&c| c > 0).collect()
    } else {
        let s: i64 = counts.iter().sum();
        if s > 0 { vec![s] } else { vec![] }
    }
}

pub fn seiten_benoetigt(counts: &[i64], fps: i64, getrennt: bool) -> i64 {
    gruppen(counts, getrennt).iter().map(|&c| aufrunden(c, fps)).sum()
}

/// Günstigste Verteilung, um `seiten_sparen` Seiten einzusparen: Ereignis-Index -> Fotos weglassen.
pub fn kuerzungsplan(counts: &[i64], fps: i64, getrennt: bool, seiten_sparen: i64) -> Vec<(usize, i64)> {
    if seiten_sparen <= 0 {
        return vec![];
    }
    let (indizes, werte): (Vec<usize>, Vec<i64>) = if getrennt {
        counts.iter().enumerate().filter(|(_, &c)| c > 0).map(|(i, &c)| (i, c)).unzip()
    } else {
        (vec![0], vec![counts.iter().sum()])
    };
    let mut kandidaten: Vec<(i64, usize)> = Vec::new();
    for (&idx, &c) in indizes.iter().zip(werte.iter()) {
        let seiten = aufrunden(c, fps);
        let rest = c % fps;
        kandidaten.push((if rest != 0 { rest } else { fps }, idx));
        for _ in 0..(seiten - 1) {
            kandidaten.push((fps, idx));
        }
    }
    kandidaten.sort();
    // Reihenfolge der Schlüssel wie bei einem Python-Dict: Einfügereihenfolge
    let mut plan: Vec<(usize, i64)> = Vec::new();
    for (kosten, idx) in kandidaten.into_iter().take(seiten_sparen as usize) {
        match plan.iter_mut().find(|(i, _)| *i == idx) {
            Some(e) => e.1 += kosten,
            None => plan.push((idx, kosten)),
        }
    }
    plan
}

/// Seitenzähler für die App.
pub fn seitenplan_ereignisse(counts: &[i64], fps: i64, profil: &Profil, getrennt: bool) -> Value {
    let fps = fps.max(1);
    let total: i64 = counts.iter().sum();
    let benoetigt = seiten_benoetigt(counts, fps, getrennt);
    let buch = erlaubte_seitenzahl(benoetigt.max(1), profil);
    let kleinere = buch - profil.schritt();
    let mut weglassen: Option<i64> = None;
    let mut plan: Vec<(usize, i64)> = vec![];
    if kleinere >= profil.min() && benoetigt > kleinere {
        plan = kuerzungsplan(counts, fps, getrennt, benoetigt - kleinere);
        weglassen = Some(plan.iter().map(|(_, v)| v).sum());
    }
    let mut plan_json = serde_json::Map::new();
    for (k, v) in &plan {
        plan_json.insert(k.to_string(), json!(v));
    }
    json!({
        "fotos": total,
        "fotos_pro_seite": fps,
        "ereignisse_getrennt": getrennt,
        "seiten_benoetigt": benoetigt,
        "buch_seiten": buch,
        "leere_seiten": (buch - benoetigt).max(0),
        "ueber_maximum": buch > profil.max(),
        "fehlende_fotos": (buch * fps - total).max(0),
        "kleinere_stufe": if weglassen.is_some() { json!(kleinere) } else { Value::Null },
        "weglassen_fuer_kleinere_stufe": weglassen,
        "kuerzungsplan": plan_json,
        "seiten_je_ereignis": counts.iter().map(|&c| if c > 0 { aufrunden(c, fps) } else { 0 }).collect::<Vec<_>>(),
    })
}

/// Seiten mit der Zahl ihrer Fotos: (Ereignis-Index oder -1, Fotos je Seite)
pub fn seitenzuordnung(counts: &[i64], fps: i64, getrennt: bool) -> Vec<(i64, i64)> {
    let fps = fps.max(1);
    let mut seiten = Vec::new();
    if getrennt {
        for (i, &c) in counts.iter().enumerate() {
            for n in verteile(c, fps) {
                seiten.push((i as i64, n));
            }
        }
    } else {
        for n in verteile(counts.iter().sum(), fps) {
            seiten.push((-1, n));
        }
    }
    seiten
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Profil {
        lade_profil("allgemein").unwrap().mit_vorgaben(None, None, None)
    }

    #[test]
    fn beispiel_aus_der_demo() {
        // Gleiche Zahlen wie im Python-Test: 5 Ereignisse mit 31,38,52,41,22 Fotos bei 3 pro Seite
        let plan = seitenplan_ereignisse(&[31, 38, 52, 41, 22], 3, &p(), true);
        assert_eq!(plan["seiten_benoetigt"], 64);
        assert_eq!(plan["fehlende_fotos"], 8);
        assert_eq!(plan["weglassen_fuer_kleinere_stufe"], 2);
        assert_eq!(plan["kuerzungsplan"], json!({"0": 1, "2": 1}));
        let plan = seitenplan_ereignisse(&[31, 38, 52, 41, 22], 3, &p(), false);
        assert_eq!(plan["seiten_benoetigt"], 62);
        assert_eq!(plan["fehlende_fotos"], 2);
        assert_eq!(plan["weglassen_fuer_kleinere_stufe"], 4);
    }

    #[test]
    fn verteilung_ist_gleichmaessig() {
        assert_eq!(verteile(7, 3), vec![3, 2, 2]);
        assert_eq!(verteile(0, 3), Vec::<i64>::new());
        assert_eq!(erlaubte_seitenzahl(25, &p()), 26);
        assert_eq!(erlaubte_seitenzahl(5, &p()), 24);
    }
}
