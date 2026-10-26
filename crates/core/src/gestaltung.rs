//! Gestaltung des Buchs: Themen (Farben, Hintergrund, Rahmen, Schrift, Deckblatt) und die Wahl des Benutzers.
//! Reine Daten und Regeln, ohne Bildverarbeitung. Gezeichnet wird in `thema_render`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub type Farbe = [u8; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hintergrund {
    Flach,
    Kraft,
    Meer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rahmen {
    Keiner,
    Weiss,
    Linie,
    Polaroid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schrift {
    Poppins,
    Lora,
    Barlow,
    Caveat,
    Plex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deckblatt {
    Klar,
    Leinen,
    Nacht,
    Kraft,
    Meer,
    Galerie,
}

#[derive(Debug)]
/// Ein Thema: feste Vorgaben, die der Benutzer teilweise überschreiben kann.
pub struct Thema {
    pub id: &'static str,
    pub name: &'static str,
    pub beschreibung: &'static str,
    pub seite: Farbe,
    pub text: Farbe,
    pub akzent: Farbe,
    pub hintergrund: Hintergrund,
    pub rahmen: Rahmen,
    pub schrift: Schrift,
    pub deckblatt: Deckblatt,
    /// Platz für Fotos in Prozent der Seite: x, y, Breite, Höhe (Dekoration und Beschriftung bleiben frei).
    pub bereich: [f64; 4],
    /// Beschriftung in Grossbuchstaben
    pub gross: bool,
}

pub const THEMEN: [Thema; 6] = [
    Thema {
        id: "klar",
        name: "Klar",
        beschreibung: "Weisse Seiten, ruhige Schrift, blauer Akzent. Die Fotos stehen im Mittelpunkt.",
        seite: [255, 255, 255],
        text: [31, 42, 55],
        akzent: [47, 111, 163],
        hintergrund: Hintergrund::Flach,
        rahmen: Rahmen::Keiner,
        schrift: Schrift::Poppins,
        deckblatt: Deckblatt::Klar,
        bereich: [0.0, 0.0, 100.0, 100.0],
        gross: false,
    },
    Thema {
        id: "leinen",
        name: "Leinen",
        beschreibung: "Buch mit Stoffeinband: Deckblatt wie Leinen mit Fenster, innen helle Seiten, weisser Fotorand, Unterschriften in Kursiv.",
        seite: [251, 250, 246],
        text: [38, 48, 42],
        akzent: [127, 148, 130],
        hintergrund: Hintergrund::Flach,
        rahmen: Rahmen::Weiss,
        schrift: Schrift::Lora,
        deckblatt: Deckblatt::Leinen,
        bereich: [0.0, 0.0, 100.0, 100.0],
        gross: false,
    },
    Thema {
        id: "nacht",
        name: "Nacht",
        beschreibung: "Schwarze Seiten lassen Abendlicht und Stadtlichter leuchten. Beschriftung in schmaler Grossschrift mit einem warmen Akzent.",
        seite: [19, 20, 23],
        text: [236, 235, 230],
        akzent: [227, 168, 87],
        hintergrund: Hintergrund::Flach,
        rahmen: Rahmen::Keiner,
        schrift: Schrift::Barlow,
        deckblatt: Deckblatt::Nacht,
        bereich: [0.0, 0.0, 100.0, 100.0],
        gross: true,
    },
    Thema {
        id: "kraft",
        name: "Kraft",
        beschreibung: "Kraftpapier wie in einem handgemachten Album: Fotos mit weissem Rand, leicht schief aufgeklebt, Beschriftung mit dem Stift.",
        seite: [201, 169, 127],
        text: [59, 44, 28],
        akzent: [176, 80, 58],
        hintergrund: Hintergrund::Kraft,
        rahmen: Rahmen::Polaroid,
        schrift: Schrift::Caveat,
        deckblatt: Deckblatt::Kraft,
        bereich: [1.0, 1.0, 98.0, 96.0],
        gross: false,
    },
    Thema {
        id: "meer",
        name: "Meer",
        beschreibung: "Helle blaugraue Seiten mit ruhiger Welle am unteren Rand, Deckblatt mit Verlauf. Der Farbton bestimmt die Welle.",
        seite: [234, 243, 246],
        text: [11, 61, 72],
        akzent: [14, 107, 124],
        hintergrund: Hintergrund::Meer,
        rahmen: Rahmen::Keiner,
        schrift: Schrift::Poppins,
        deckblatt: Deckblatt::Meer,
        bereich: [0.0, 0.0, 100.0, 86.0],
        gross: false,
    },
    Thema {
        id: "galerie",
        name: "Galerie",
        beschreibung: "Museums-Look: breiter Rand, dünner Strich um jedes Foto, kleines Schild mit Nummer. Ein roter Punkt als einziges Farbsignal.",
        seite: [247, 247, 245],
        text: [17, 17, 17],
        akzent: [200, 50, 30],
        hintergrund: Hintergrund::Flach,
        rahmen: Rahmen::Linie,
        schrift: Schrift::Plex,
        deckblatt: Deckblatt::Galerie,
        bereich: [5.0, 4.0, 90.0, 88.0],
        gross: true,
    },
];

pub const HINTERGRUENDE: [&str; 6] = ["thema", "weiss", "papier", "schwarz", "kraft", "meer"];
pub const RAHMEN_NAMEN: [&str; 5] = ["thema", "keiner", "weiss", "linie", "polaroid"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Gestaltung {
    pub thema: String,
    /// Eigener Farbton als «#rrggbb»; leer = Farbton des Themas
    pub farbton: String,
    pub hintergrund: String,
    pub rahmen: String,
    pub titel: String,
    pub untertitel: String,
    pub name: String,
    /// Foto-ID des Titelfotos; leer = erstes Foto im Buch
    pub titelfoto: String,
    pub beschriftung: bool,
    pub seitenzahl: bool,
}

impl Default for Gestaltung {
    fn default() -> Self {
        Gestaltung {
            thema: "klar".into(),
            farbton: String::new(),
            hintergrund: "thema".into(),
            rahmen: "thema".into(),
            titel: String::new(),
            untertitel: String::new(),
            name: String::new(),
            titelfoto: String::new(),
            beschriftung: true,
            seitenzahl: true,
        }
    }
}

/// Aufgelöste Darstellung: Thema plus Überschreibungen des Benutzers.
#[derive(Clone, Debug)]
pub struct Stil {
    pub thema: &'static Thema,
    pub seite: Farbe,
    pub text: Farbe,
    pub akzent: Farbe,
    pub hintergrund: Hintergrund,
    pub rahmen: Rahmen,
    pub bereich: [f64; 4],
    pub beschriftung: bool,
    pub seitenzahl: bool,
}

pub fn thema(id: &str) -> &'static Thema {
    THEMEN.iter().find(|t| t.id == id).unwrap_or(&THEMEN[0])
}

pub fn farbe_aus_hex(s: &str) -> Option<Farbe> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(t, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Mischt `a` mit `b`: t = 0 ergibt `a`, t = 1 ergibt `b`.
pub fn mische_farbe(a: Farbe, b: Farbe, t: f32) -> Farbe {
    [0, 1, 2].map(|i| (a[i] as f32 * (1.0 - t) + b[i] as f32 * t).round() as u8)
}

pub fn hex(f: Farbe) -> String {
    format!("#{:02x}{:02x}{:02x}", f[0], f[1], f[2])
}

fn hell(f: Farbe) -> f64 {
    (0.2126 * f[0] as f64 + 0.7152 * f[1] as f64 + 0.0722 * f[2] as f64) / 255.0
}

impl Gestaltung {
    pub fn stil(&self) -> Stil {
        let t = thema(&self.thema);
        let akzent = farbe_aus_hex(&self.farbton).unwrap_or(t.akzent);
        let (seite, hg, vorgabe_text) = match self.hintergrund.as_str() {
            "weiss" => ([255, 255, 255], Hintergrund::Flach, None),
            "papier" => ([250, 246, 236], Hintergrund::Flach, None),
            "schwarz" => ([19, 20, 23], Hintergrund::Flach, None),
            "kraft" => ([201, 169, 127], Hintergrund::Kraft, None),
            "meer" => ([234, 243, 246], Hintergrund::Meer, None),
            _ => (t.seite, t.hintergrund, Some(t.text)),
        };
        // Schriftfarbe: die des Themas, wenn der Hintergrund ähnlich hell ist wie beim Thema, sonst automatisch
        let text = match vorgabe_text {
            Some(c) => c,
            None if (hell(seite) < 0.4) == (hell(t.seite) < 0.4) => t.text,
            None if hell(seite) < 0.4 => [236, 235, 230],
            None => [31, 42, 55],
        };
        let rahmen = match self.rahmen.as_str() {
            "keiner" => Rahmen::Keiner,
            "weiss" => Rahmen::Weiss,
            "linie" => Rahmen::Linie,
            "polaroid" => Rahmen::Polaroid,
            _ => t.rahmen,
        };
        // Bereich: Das Thema legt ihn fest; eine andere Wahl passt ihn an (Welle braucht unten Platz, Polaroid etwas Rand)
        let mut bereich = t.bereich;
        if hg == Hintergrund::Meer && t.hintergrund != Hintergrund::Meer {
            bereich = [0.0, 0.0, 100.0, 86.0];
        } else if hg != Hintergrund::Meer && t.hintergrund == Hintergrund::Meer {
            bereich = [0.0, 0.0, 100.0, 100.0];
        }
        if rahmen == Rahmen::Linie && t.rahmen != Rahmen::Linie {
            bereich = [bereich[0] + 4.0, bereich[1] + 3.0, bereich[2] - 8.0, bereich[3] - 6.0];
        }
        Stil { thema: t, seite, text, akzent, hintergrund: hg, rahmen, bereich, beschriftung: self.beschriftung, seitenzahl: self.seitenzahl }
    }

    /// Änderungen prüfen und übernehmen. Unbekannte Felder oder Werte sind Fehler.
    pub fn setze(&mut self, neu: &serde_json::Map<String, Value>) -> Result<(), String> {
        let mut g = self.clone();
        for (k, v) in neu {
            let text = |v: &Value| -> Result<String, String> { v.as_str().map(|s| s.to_string()).ok_or_else(|| format!("Ungültiger Wert für {k}")) };
            let wahr = |v: &Value| -> Result<bool, String> { v.as_bool().ok_or_else(|| format!("Ungültiger Wert für {k}")) };
            match k.as_str() {
                "thema" => {
                    let t = text(v)?;
                    if !THEMEN.iter().any(|x| x.id == t) {
                        return Err("Unbekanntes Thema".into());
                    }
                    g.thema = t;
                }
                "farbton" => {
                    let t = text(v)?;
                    if !t.is_empty() && farbe_aus_hex(&t).is_none() {
                        return Err("Der Farbton muss wie «#2f6fa3» geschrieben sein.".into());
                    }
                    g.farbton = if t.is_empty() { t } else { hex(farbe_aus_hex(&t).unwrap()) };
                }
                "hintergrund" => {
                    let t = text(v)?;
                    if !HINTERGRUENDE.contains(&t.as_str()) {
                        return Err("Unbekannter Hintergrund".into());
                    }
                    g.hintergrund = t;
                }
                "rahmen" => {
                    let t = text(v)?;
                    if !RAHMEN_NAMEN.contains(&t.as_str()) {
                        return Err("Unbekannter Fotorahmen".into());
                    }
                    g.rahmen = t;
                }
                "titel" | "untertitel" | "name" => {
                    let t: String = text(v)?.chars().filter(|c| !c.is_control()).take(80).collect();
                    match k.as_str() {
                        "titel" => g.titel = t,
                        "untertitel" => g.untertitel = t,
                        _ => g.name = t,
                    }
                }
                "titelfoto" => g.titelfoto = text(v)?.chars().take(40).collect(),
                "beschriftung" => g.beschriftung = wahr(v)?,
                "seitenzahl" => g.seitenzahl = wahr(v)?,
                _ => return Err(format!("Unbekannte Einstellung: {k}")),
            }
        }
        *self = g;
        Ok(())
    }

    /// Wahl zurück auf das Thema (Farbton, Hintergrund, Rahmen), Texte und Titelfoto bleiben.
    pub fn zurueck_auf_thema(&mut self) {
        self.farbton.clear();
        self.hintergrund = "thema".into();
        self.rahmen = "thema".into();
    }
}

fn css_hg(h: Hintergrund) -> &'static str {
    match h {
        Hintergrund::Flach => "flach",
        Hintergrund::Kraft => "kraft",
        Hintergrund::Meer => "meer",
    }
}

fn css_rahmen(r: Rahmen) -> &'static str {
    match r {
        Rahmen::Keiner => "keiner",
        Rahmen::Weiss => "weiss",
        Rahmen::Linie => "linie",
        Rahmen::Polaroid => "polaroid",
    }
}

/// Für die Oberfläche: gewählte Werte, aufgelöster Stil und die Liste der Themen.
pub fn ansicht(g: &Gestaltung, titel_auto: &str, untertitel_auto: &str, titelfoto: Option<&str>, kandidaten: &[String]) -> Value {
    let s = g.stil();
    json!({
        "wahl": g,
        "stil": {
            "seite": hex(s.seite), "text": hex(s.text), "akzent": hex(s.akzent),
            "hintergrund": css_hg(s.hintergrund), "rahmen": css_rahmen(s.rahmen),
            "bereich": s.bereich, "gross": s.thema.gross, "deckblatt": s.thema.id,
            "beschriftung": s.beschriftung, "seitenzahl": s.seitenzahl,
        },
        "titel_auto": titel_auto,
        "untertitel_auto": untertitel_auto,
        "titelfoto": titelfoto,
        "kandidaten": kandidaten,
        "themen": THEMEN.iter().map(|t| json!({
            "id": t.id, "name": t.name, "beschreibung": t.beschreibung,
            "seite": hex(t.seite), "text": hex(t.text), "akzent": hex(t.akzent),
        })).collect::<Vec<_>>(),
        "hintergruende": HINTERGRUENDE,
        "rahmen_namen": RAHMEN_NAMEN,
    })
}

/// Fotofeld (Prozent der Seite) in den Bereich des Themas legen. Randlose Felder und Felder über die Doppelseite bleiben unverändert.
pub fn feld_im_bereich(f: [f64; 4], bereich: [f64; 4]) -> [f64; 4] {
    if am_rand(f) {
        return f;
    }
    [bereich[0] + f[0] * bereich[2] / 100.0, bereich[1] + f[1] * bereich[3] / 100.0, f[2] * bereich[2] / 100.0, f[3] * bereich[3] / 100.0]
}

/// Ein Feld, das bis an den Seitenrand reicht oder darüber hinaus (randlos, Panorama über die Doppelseite).
pub fn am_rand(f: [f64; 4]) -> bool {
    f[0] <= 0.5 || f[1] <= 0.5 || f[0] + f[2] >= 99.5 || f[1] + f[3] >= 99.5
}

pub fn monat_name(m: u32) -> &'static str {
    ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"][(m.clamp(1, 12) - 1) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_ist_klar_und_aendert_nichts() {
        let g = Gestaltung::default();
        let s = g.stil();
        assert_eq!(s.thema.id, "klar");
        assert_eq!(s.seite, [255, 255, 255]);
        assert_eq!(s.rahmen, Rahmen::Keiner);
        assert_eq!(s.bereich, [0.0, 0.0, 100.0, 100.0]);
    }

    #[test]
    fn ueberschreibungen_gelten_und_schrift_bleibt_lesbar() {
        let mut g = Gestaltung::default();
        g.setze(json!({"hintergrund": "schwarz", "farbton": "#AA3300", "rahmen": "linie"}).as_object().unwrap()).unwrap();
        let s = g.stil();
        assert_eq!(s.seite, [19, 20, 23]);
        assert!(hell(s.text) > 0.8, "auf Schwarz helle Schrift");
        assert_eq!(s.akzent, [0xaa, 0x33, 0x00]);
        assert_eq!(s.rahmen, Rahmen::Linie);
        assert!(s.bereich[0] > 0.0, "Linie braucht Rand");
        g.zurueck_auf_thema();
        assert_eq!(g.stil().seite, [255, 255, 255]);
    }

    #[test]
    fn ungueltiges_wird_abgelehnt_und_aendert_nichts() {
        let mut g = Gestaltung::default();
        assert!(g.setze(json!({"thema": "gibtsnicht"}).as_object().unwrap()).is_err());
        assert!(g.setze(json!({"thema": "nacht", "farbton": "rot"}).as_object().unwrap()).is_err());
        assert_eq!(g, Gestaltung::default(), "alles oder nichts");
        assert!(g.setze(json!({"unbekannt": 1}).as_object().unwrap()).is_err());
    }

    #[test]
    fn bereich_verschiebt_normale_felder_aber_nicht_randlose() {
        let b = [5.0, 4.0, 90.0, 88.0];
        let f = feld_im_bereich([10.0, 10.0, 50.0, 50.0], b);
        assert!((f[0] - 14.0).abs() < 1e-9 && (f[2] - 45.0).abs() < 1e-9);
        assert_eq!(feld_im_bereich([0.0, 0.0, 100.0, 100.0], b), [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(feld_im_bereich([-100.0, 0.0, 200.0, 100.0], b), [-100.0, 0.0, 200.0, 100.0]);
    }
}
