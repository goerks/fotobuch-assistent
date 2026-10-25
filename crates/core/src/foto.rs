//! Das Foto mit allen eingelesenen Eigenschaften. Das Format der gespeicherten Datei ist
//! dasselbe wie in der früheren Python-Version, damit bestehende Projekte weiter funktionieren.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Foto {
    /// Pfad relativ zum Fotoordner, immer mit «/» geschrieben.
    pub pfad: String,
    pub name: String,
    pub endung: String,
    pub groesse_bytes: u64,
    pub zeit: NaiveDateTime,
    /// «exif» oder «datei»
    pub zeit_quelle: String,
    #[serde(default)]
    pub breite: u32,
    #[serde(default)]
    pub hoehe: u32,
    #[serde(default)]
    pub kamera: String,
    #[serde(default)]
    pub gps: Option<(f64, f64)>,
    #[serde(default)]
    pub sha256: String,
    /// 64-Bit-Differenzhash, als 16 Hex-Zeichen gespeichert
    #[serde(default, with = "hex64")]
    pub dhash: u64,
    #[serde(default)]
    pub schaerfe: f64,
    #[serde(default)]
    pub helligkeit: f64,
    #[serde(default)]
    pub ist_raw: bool,
    #[serde(default)]
    pub screenshot_verdacht: bool,
    #[serde(default)]
    pub fehler: String,
    #[serde(default)]
    pub hinweise: Vec<String>,
}

mod hex64 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:016x}", v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let t = String::deserialize(d)?;
        u64::from_str_radix(&t, 16).map_err(serde::de::Error::custom)
    }
}

impl Foto {
    pub fn format(&self) -> &'static str {
        if self.breite == 0 || self.hoehe == 0 {
            return "unbekannt";
        }
        let (b, h) = (self.breite as f64, self.hoehe as f64);
        if b > h * 1.02 {
            "quer"
        } else if h > b * 1.02 {
            "hoch"
        } else {
            "quadrat"
        }
    }

    pub fn megapixel(&self) -> f64 {
        self.breite as f64 * self.hoehe as f64 / 1e6
    }

    /// Einheitliche Reihenfolge: Zeit, Name, Pfad.
    pub fn schluessel(&self) -> (NaiveDateTime, String, String) {
        (self.zeit, self.name.to_lowercase(), self.pfad.to_lowercase())
    }
}
