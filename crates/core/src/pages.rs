//! Druckdienst-Profile und erlaubte Seitenzahlen. Die Verteilung der Fotos auf Seiten steht in `umfang`.

use serde::Deserialize;

/// Ein Anbieter mit seinen Produkten (aus `profiles/*.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct Anbieter {
    pub name: String,
    #[serde(default)]
    pub quelle: Vec<String>,
    pub geprueft_am: Option<String>,
    #[serde(default)]
    pub hinweis: String,
    pub produkte: Vec<Produkt>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Produkt {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub verifiziert: bool,
    pub seiten_min: Option<i64>,
    pub seiten_schritt: Option<i64>,
    pub seiten_max: Option<i64>,
    /// Feste Seitenzahlen (statt Bereich mit Schritt).
    pub seiten_liste: Option<Vec<i64>>,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub hinweis: String,
}

/// Die Seitenregeln, mit denen gerechnet wird (Anbieter und Produkt, dazu eigene Anpassungen).
#[derive(Clone, Debug)]
pub struct Profil {
    pub name: String,
    pub produkt: String,
    pub verifiziert: bool,
    pub seiten_min: Option<i64>,
    pub seiten_schritt: Option<i64>,
    pub seiten_max: Option<i64>,
    pub seiten_liste: Option<Vec<i64>>,
    pub hinweis: String,
    pub quelle: Vec<String>,
    pub geprueft_am: Option<String>,
    pub format: String,
}

pub const PROFILE: [&str; 5] = ["allgemein", "cewe", "pixum", "ifolor", "saal-digital"];

pub fn lade_anbieter(name: &str) -> Option<Anbieter> {
    let text = match name {
        "allgemein" => include_str!("../profiles/allgemein.json"),
        "cewe" | "cewe-pixum" => include_str!("../profiles/cewe.json"),
        "pixum" => include_str!("../profiles/pixum.json"),
        "ifolor" => include_str!("../profiles/ifolor.json"),
        "saal-digital" => include_str!("../profiles/saal-digital.json"),
        _ => return None,
    };
    serde_json::from_str(text).ok()
}

/// Profil eines Anbieters; ein unbekanntes oder leeres Produkt ergibt das erste.
pub fn lade_profil_produkt(name: &str, produkt: &str) -> Option<Profil> {
    let a = lade_anbieter(name)?;
    let p = a.produkte.iter().find(|p| p.id == produkt).or_else(|| a.produkte.first())?.clone();
    let hinweis = [a.hinweis.as_str(), p.hinweis.as_str()].iter().filter(|t| !t.is_empty()).cloned().collect::<Vec<_>>().join(" ");
    Some(Profil {
        name: a.name,
        produkt: p.id,
        verifiziert: p.verifiziert,
        seiten_min: p.seiten_min,
        seiten_schritt: p.seiten_schritt,
        seiten_max: p.seiten_max,
        seiten_liste: p.seiten_liste,
        hinweis,
        quelle: a.quelle,
        geprueft_am: a.geprueft_am,
        format: p.format,
    })
}

pub fn lade_profil(name: &str) -> Option<Profil> {
    lade_profil_produkt(name, "")
}

impl Profil {
    /// Eigene Zahlen ersetzen die Regeln des Produkts (feste Seitenzahlen fallen dann weg).
    pub fn mit_vorgaben(&self, min: Option<i64>, schritt: Option<i64>, max: Option<i64>) -> Profil {
        let eigen = min.is_some() || schritt.is_some() || max.is_some();
        Profil {
            seiten_min: Some(min.unwrap_or(self.min())),
            seiten_schritt: Some(schritt.unwrap_or(self.schritt())),
            seiten_max: Some(max.unwrap_or(self.max())),
            seiten_liste: if eigen { None } else { self.seiten_liste.clone() },
            ..self.clone()
        }
    }
    pub fn min(&self) -> i64 {
        match &self.seiten_liste {
            Some(l) if !l.is_empty() => l[0],
            _ => self.seiten_min.unwrap_or(24),
        }
    }
    pub fn schritt(&self) -> i64 {
        match &self.seiten_liste {
            Some(l) if l.len() > 1 => l.windows(2).map(|w| w[1] - w[0]).min().unwrap_or(2).max(1),
            _ => self.seiten_schritt.unwrap_or(2),
        }
    }
    pub fn max(&self) -> i64 {
        match &self.seiten_liste {
            Some(l) if !l.is_empty() => *l.last().unwrap(),
            _ => self.seiten_max.unwrap_or(200),
        }
    }
    /// Nächstkleinere erlaubte Seitenzahl unter `buch`.
    pub fn kleiner_als(&self, buch: i64) -> Option<i64> {
        match &self.seiten_liste {
            Some(l) => l.iter().rev().copied().find(|&x| x < buch),
            None => (buch - self.schritt() >= self.min()).then_some(buch - self.schritt()),
        }
    }
    /// Nächstgrössere erlaubte Seitenzahl über `buch`.
    pub fn groesser_als(&self, buch: i64) -> Option<i64> {
        match &self.seiten_liste {
            Some(l) => l.iter().copied().find(|&x| x > buch),
            None => (buch + self.schritt() <= self.max()).then_some(buch + self.schritt()),
        }
    }
    /// Die Regel in einem Satz für die Oberfläche.
    pub fn regel_text(&self) -> String {
        match &self.seiten_liste {
            Some(l) => format!("Feste Seitenzahlen: {} Seiten", l.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")),
            None => {
                let (mn, s, mx) = (self.min(), self.schritt(), self.max());
                format!("{mn} bis {mx} Seiten, in {s}er-Schritten ({mn}, {}, {}, …)", mn + s, mn + 2 * s)
            }
        }
    }
}

pub(crate) fn aufrunden(a: i64, b: i64) -> i64 {
    (a + b - 1).div_euclid(b)
}

/// Kleinste erlaubte Seitenzahl, die mindestens `benoetigt` Seiten bietet.
pub fn erlaubte_seitenzahl(benoetigt: i64, p: &Profil) -> i64 {
    if let Some(l) = p.seiten_liste.as_ref().filter(|l| !l.is_empty()) {
        // Feste Seitenzahlen: die nächste passende, bei zu vielen Seiten die Zahl selbst (dann warnt der Zähler)
        return l.iter().copied().find(|&x| x >= benoetigt).unwrap_or(benoetigt);
    }
    let (mn, schritt) = (p.min(), p.schritt());
    if benoetigt <= mn {
        return mn;
    }
    mn + aufrunden(benoetigt - mn, schritt) * schritt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Profil {
        lade_profil("allgemein").unwrap().mit_vorgaben(None, None, None)
    }

    #[test]
    fn erlaubte_seitenzahlen() {
        assert_eq!(erlaubte_seitenzahl(25, &p()), 26);
        assert_eq!(erlaubte_seitenzahl(5, &p()), 24);
        assert_eq!(erlaubte_seitenzahl(24, &p()), 24);
    }

    #[test]
    fn echte_profile() {
        let pixum = lade_profil("pixum").unwrap();
        assert_eq!((pixum.min(), pixum.schritt(), pixum.max()), (26, 4, 202));
        assert_eq!(erlaubte_seitenzahl(27, &pixum), 30);
        assert_eq!(erlaubte_seitenzahl(30, &pixum), 30);
        assert_eq!(pixum.regel_text(), "26 bis 202 Seiten, in 4er-Schritten (26, 30, 34, …)");
        let saal = lade_profil("saal-digital").unwrap();
        assert_eq!(erlaubte_seitenzahl(27, &saal), 28);
        let ifolor = lade_profil("ifolor").unwrap();
        assert_eq!(erlaubte_seitenzahl(5, &ifolor), 26);
        assert_eq!(erlaubte_seitenzahl(27, &ifolor), 36);
        assert_eq!(erlaubte_seitenzahl(133, &ifolor), 133, "über dem Maximum: Zähler warnt");
        assert_eq!(ifolor.kleiner_als(36), Some(26));
        assert_eq!(ifolor.groesser_als(132), None);
        assert_eq!(ifolor.groesser_als(36), Some(48));
        // eigene Zahlen ersetzen die feste Liste
        let eigen = ifolor.mit_vorgaben(Some(20), None, None);
        assert!(eigen.seiten_liste.is_none());
        // alter Name bleibt gültig
        assert_eq!(lade_profil("cewe-pixum").unwrap().name, "Cewe Fotobuch");
        // Produktwahl
        let h = lade_profil_produkt("pixum", "hardcover-layflat").unwrap();
        assert_eq!(h.max(), 130);
        for n in PROFILE {
            assert!(lade_profil(n).is_some(), "{n}");
        }
    }
}
