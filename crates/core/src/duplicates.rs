//! Duplikate und Serien finden. Stufe 0: Datei-Hash und Differenzhash, keine KI.

use std::collections::HashMap;

use crate::foto::Foto;

#[derive(Clone, Debug)]
pub struct Gruppe {
    /// Indizes in die Fotoliste
    pub fotos: Vec<usize>,
    pub vorschlag: usize,
}

pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn bewertung(f: &Foto, max_schaerfe: f64) -> f64 {
    let s = if max_schaerfe > 0.0 { f.schaerfe / max_schaerfe } else { 0.0 };
    0.7 * s + 0.3 * (f.megapixel() / 12.0).min(1.0)
}

/// Bestes Foto der Gruppe: Schärfe zählt am meisten, dann Auflösung. Bei Gleichstand das erste.
pub fn waehle_vorschlag(fotos: &[Foto], gruppe: &[usize]) -> usize {
    let maxs = gruppe.iter().map(|&i| fotos[i].schaerfe).fold(0.0f64, f64::max);
    let mut beste = gruppe[0];
    let mut bestes = (bewertung(&fotos[beste], maxs), fotos[beste].groesse_bytes);
    for &i in &gruppe[1..] {
        let w = (bewertung(&fotos[i], maxs), fotos[i].groesse_bytes);
        if w.0 > bestes.0 || (w.0 == bestes.0 && w.1 > bestes.1) {
            beste = i;
            bestes = w;
        }
    }
    beste
}

fn nach_schluessel(fotos: &[Foto], liste: &mut Vec<usize>) {
    liste.sort_by_key(|&i| fotos[i].schluessel());
}

pub fn finde_exakte(fotos: &[Foto]) -> Vec<Gruppe> {
    let mut nach_hash: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, f) in fotos.iter().enumerate() {
        if !f.sha256.is_empty() && f.fehler.is_empty() {
            nach_hash.entry(f.sha256.as_str()).or_default().push(i);
        }
    }
    let mut gruppen: Vec<Gruppe> = nach_hash
        .into_values()
        .filter(|l| l.len() > 1)
        .map(|mut l| {
            nach_schluessel(fotos, &mut l);
            let vorschlag = l[0];
            Gruppe { fotos: l, vorschlag }
        })
        .collect();
    gruppen.sort_by_key(|g| fotos[g.fotos[0]].schluessel());
    gruppen
}

pub struct SerienOptionen {
    pub max_abstand: u32,
    pub fenster_sekunden: f64,
    pub max_spanne_sekunden: Option<f64>,
    pub fenster_ort_sekunden: f64,
}

/// Serien: ähnlicher Differenzhash und kurzer zeitlicher Abstand.
///
/// Am selben Ort darf der zeitliche Abstand bis `fenster_ort_sekunden` betragen, an verschiedenen Orten
/// bilden Fotos nie eine Serie. Fehlt bei einem Foto der Ort, gilt das normale Fenster. Exakte Duplikate
/// werden nur mit einem Vertreter geführt.
pub fn finde_aehnliche(fotos: &[Foto], kandidaten: &[usize], opt: &SerienOptionen, ort_von: &[Option<usize>]) -> Vec<Gruppe> {
    let max_spanne = opt.max_spanne_sekunden.unwrap_or(opt.fenster_sekunden * 2.0);
    let mut sortiert: Vec<usize> = kandidaten.iter().copied().filter(|&i| fotos[i].fehler.is_empty() && fotos[i].breite > 0).collect();
    nach_schluessel(fotos, &mut sortiert);
    let mut gesehen: std::collections::HashSet<&str> = Default::default();
    let mut vertreter: Vec<usize> = Vec::new();
    for i in sortiert {
        let h = fotos[i].sha256.as_str();
        if !h.is_empty() && gesehen.contains(h) {
            continue;
        }
        if !h.is_empty() {
            gesehen.insert(h);
        }
        vertreter.push(i);
    }

    let hat_orte = ort_von.iter().any(|o| o.is_some());
    let grosses_fenster = if hat_orte { opt.fenster_sekunden.max(opt.fenster_ort_sekunden) } else { opt.fenster_sekunden };
    let grenzen = |a: usize, b: usize| -> Option<(f64, f64)> {
        match (ort_von[a], ort_von[b]) {
            (Some(oa), Some(ob)) => {
                if oa != ob {
                    None
                } else {
                    Some((opt.fenster_ort_sekunden, max_spanne.max(2.0 * opt.fenster_ort_sekunden)))
                }
            }
            _ => Some((opt.fenster_sekunden, max_spanne)),
        }
    };

    let mut alle: Vec<Vec<usize>> = Vec::new();
    let mut offene: Vec<usize> = Vec::new(); // Indizes in `alle`
    for &f in &vertreter {
        let zf = fotos[f].zeit;
        offene.retain(|&c| (zf - fotos[*alle[c].last().unwrap()].zeit).num_milliseconds() as f64 / 1000.0 <= grosses_fenster);
        let mut ziel: Option<usize> = None;
        'suche: for &c in &offene {
            for &m in &alle[c] {
                let Some((fenster, spanne)) = grenzen(f, m) else { continue };
                if (zf - fotos[m].zeit).num_milliseconds() as f64 / 1000.0 > fenster {
                    continue;
                }
                if (zf - fotos[alle[c][0]].zeit).num_milliseconds() as f64 / 1000.0 > spanne {
                    continue;
                }
                if hamming(fotos[f].dhash, fotos[m].dhash) <= opt.max_abstand {
                    ziel = Some(c);
                    break 'suche;
                }
            }
        }
        match ziel {
            Some(c) => alle[c].push(f),
            None => {
                alle.push(vec![f]);
                offene.push(alle.len() - 1);
            }
        }
    }

    let mut gruppen: Vec<Gruppe> = alle
        .into_iter()
        .filter(|l| l.len() > 1)
        .map(|l| {
            let v = waehle_vorschlag(fotos, &l);
            Gruppe { fotos: l, vorschlag: v }
        })
        .collect();
    gruppen.sort_by_key(|g| fotos[g.fotos[0]].zeit);
    gruppen
}
