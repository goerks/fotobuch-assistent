//! Orte bilden: Fotos, die an derselben Stelle entstanden sind, über GPS zusammenfassen.
//!
//! Stufe 0, ohne KI und ohne Internet: Es gibt keine Ortsnamen, nur Koordinaten. Fotos ohne GPS
//! werden dem Ort zugeordnet, an dem das zeitlich nächste Foto mit GPS entstand, sofern es nicht
//! zu lange her ist.

use crate::foto::Foto;
use crate::scanner::entfernung_km;

#[derive(Clone, Debug)]
pub struct Ort {
    pub nummer: usize,
    /// Indizes in die Fotoliste, nach Zeit sortiert
    pub fotos: Vec<usize>,
    pub lat: f64,
    pub lon: f64,
    pub mit_gps: usize,
}

/// Gibt die Orte und für jedes Foto (Index) den Index seines Ortes zurück.
pub fn bilde_orte(fotos: &[Foto], radius_m: f64, zeit_toleranz_minuten: f64) -> (Vec<Ort>, Vec<Option<usize>>) {
    let mut mit: Vec<usize> = (0..fotos.len()).filter(|&i| fotos[i].gps.is_some() && fotos[i].fehler.is_empty()).collect();
    mit.sort_by(|&a, &b| (fotos[a].zeit, fotos[a].name.to_lowercase()).cmp(&(fotos[b].zeit, fotos[b].name.to_lowercase())));
    let mut orte: Vec<Ort> = Vec::new();
    let mut summen: Vec<(f64, f64)> = Vec::new();
    let mut zuordnung: Vec<Option<usize>> = vec![None; fotos.len()];

    for &i in &mit {
        let g = fotos[i].gps.unwrap();
        let mut bester: Option<(usize, f64)> = None;
        for (k, o) in orte.iter().enumerate() {
            let d = entfernung_km(g, (o.lat, o.lon)) * 1000.0;
            if d <= radius_m && bester.map_or(true, |(_, bd)| d < bd) {
                bester = Some((k, d));
            }
        }
        let k = match bester {
            Some((k, _)) => k,
            None => {
                orte.push(Ort { nummer: orte.len() + 1, fotos: vec![], lat: g.0, lon: g.1, mit_gps: 0 });
                summen.push((0.0, 0.0));
                orte.len() - 1
            }
        };
        let o = &mut orte[k];
        o.fotos.push(i);
        o.mit_gps += 1;
        summen[k].0 += g.0;
        summen[k].1 += g.1;
        o.lat = summen[k].0 / o.mit_gps as f64;
        o.lon = summen[k].1 / o.mit_gps as f64;
        zuordnung[i] = Some(k);
    }

    if !mit.is_empty() {
        let toleranz = zeit_toleranz_minuten * 60.0;
        for i in 0..fotos.len() {
            let f = &fotos[i];
            if zuordnung[i].is_some() || !f.fehler.is_empty() || f.screenshot_verdacht {
                continue;
            }
            // Platz in der nach Zeit sortierten GPS-Liste (bisect_left)
            let pos = mit.partition_point(|&m| fotos[m].zeit < f.zeit);
            let mut kand = Vec::new();
            if pos > 0 {
                kand.push(pos - 1);
            }
            if pos < mit.len() {
                kand.push(pos);
            }
            let j = *kand
                .iter()
                .min_by_key(|&&k| (fotos[mit[k]].zeit - f.zeit).num_seconds().abs())
                .unwrap();
            if ((fotos[mit[j]].zeit - f.zeit).num_seconds().abs() as f64) <= toleranz {
                let idx = zuordnung[mit[j]].unwrap();
                orte[idx].fotos.push(i);
                zuordnung[i] = Some(idx);
            }
        }
    }

    // Orte nach frühestem Foto ordnen, Nummern neu vergeben
    let start = |o: &Ort| o.fotos.iter().map(|&i| fotos[i].zeit).min().unwrap();
    let mut reihenfolge: Vec<usize> = (0..orte.len()).collect();
    reihenfolge.sort_by_key(|&i| start(&orte[i]));
    let mut neu_von_alt = vec![0usize; orte.len()];
    for (n, &alt) in reihenfolge.iter().enumerate() {
        neu_von_alt[alt] = n;
    }
    let mut sortiert: Vec<Ort> = reihenfolge.iter().map(|&i| orte[i].clone()).collect();
    for (n, o) in sortiert.iter_mut().enumerate() {
        o.nummer = n + 1;
        o.fotos.sort_by(|&a, &b| (fotos[a].zeit, fotos[a].name.to_lowercase()).cmp(&(fotos[b].zeit, fotos[b].name.to_lowercase())));
    }
    let zuordnung = zuordnung.into_iter().map(|z| z.map(|a| neu_von_alt[a])).collect();
    (sortiert, zuordnung)
}
