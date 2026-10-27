//! Druckdienst-Profile und erlaubte Seitenzahlen. Die Verteilung der Fotos auf Seiten steht in `umfang`.

use serde::Deserialize;

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

pub(crate) fn aufrunden(a: i64, b: i64) -> i64 {
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
}
