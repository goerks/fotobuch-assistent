//! Lokaler Server der App. Hört nur auf 127.0.0.1 (zufälliger Port), Zugriff nur mit Token.
//! Nichts verlässt den Rechner. Die Oberfläche (`ui/`) ist in das Programm eingebettet.

use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rust_embed::RustEmbed;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::projekt::{analysieren, vorschau_bytes, Projekt, Res};

#[derive(RustEmbed)]
#[folder = "../../ui/"]
struct Ui;

const MAX_BODY: usize = 1_000_000;

#[derive(Clone, Debug)]
pub struct Job {
    pub laeuft: bool,
    pub phase: String,
    pub fertig: usize,
    pub gesamt: usize,
    pub fehler: Option<String>,
    pub art: String,
    pub ergebnis: Option<Value>,
}

impl Job {
    fn leer() -> Job {
        Job { laeuft: false, phase: String::new(), fertig: 0, gesamt: 0, fehler: None, art: String::new(), ergebnis: None }
    }
    fn json(&self) -> Value {
        json!({"laeuft": self.laeuft, "phase": self.phase, "fertig": self.fertig, "gesamt": self.gesamt, "fehler": self.fehler, "art": self.art, "ergebnis": self.ergebnis})
    }
}

pub type OrdnerDialog = Box<dyn Fn() -> Option<String> + Send + Sync>;

pub struct App {
    pub token: String,
    pub projekt: Mutex<Option<Arc<Mutex<Projekt>>>>,
    pub job: Arc<Mutex<Job>>,
    pub ordner_dialog: Option<OrdnerDialog>,
    pub beenden: AtomicBool,
    /// true, wenn die Oberfläche in einem eigenen App-Fenster läuft (Links werden dann extern geöffnet).
    pub nativ: bool,
}

fn zufalls_token() -> String {
    let mut b = [0u8; 24];
    getrandom::getrandom(&mut b).expect("Zufallszahlen");
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

impl App {
    pub fn neu(ordner_dialog: Option<OrdnerDialog>, nativ: bool) -> Arc<App> {
        Arc::new(App {
            token: zufalls_token(),
            projekt: Mutex::new(None),
            job: Arc::new(Mutex::new(Job::leer())),
            ordner_dialog,
            beenden: AtomicBool::new(false),
            nativ,
        })
    }

    pub fn oeffne(&self, ordner: &str) -> Res<()> {
        if self.job.lock().unwrap().laeuft {
            return Err("Es läuft gerade eine Analyse.".into());
        }
        let p = Projekt::neu(&PathBuf::from(ordner))?;
        *self.projekt.lock().unwrap() = Some(Arc::new(Mutex::new(p)));
        *self.job.lock().unwrap() = Job::leer();
        Ok(())
    }

    fn aktuelles(&self) -> Res<Arc<Mutex<Projekt>>> {
        self.projekt.lock().unwrap().clone().ok_or_else(|| "Noch kein Ordner gewählt.".to_string())
    }

    /// Rendert die fertigen Seiten (JPEG + PDF) im Hintergrund. Fortschritt und Ergebnis stehen im Job.
    pub fn starte_seitenexport(&self, ziel: Option<String>) -> Res<()> {
        let projekt = self.aktuelles()?;
        let (paket, ordner) = {
            let p = projekt.lock().map_err(|_| "Interner Fehler")?;
            (p.seitenauftraege()?, p.seiten_zielordner(ziel.as_deref()))
        };
        {
            let mut j = self.job.lock().unwrap();
            if j.laeuft {
                return Err("Es läuft gerade eine andere Aufgabe.".into());
            }
            *j = Job { laeuft: true, phase: "seiten".into(), fertig: 0, gesamt: paket.auftraege.len() + paket.leere + 1, fehler: None, art: "seiten".into(), ergebnis: None };
        }
        let job = self.job.clone();
        std::thread::spawn(move || {
            let job2 = job.clone();
            let fortschritt = move |fertig: usize, gesamt: usize| {
                if let Ok(mut j) = job2.lock() {
                    j.fertig = fertig;
                    j.gesamt = gesamt;
                }
            };
            let erg = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::seitenexport::exportiere(&paket.auftraege, Some(&paket.deckblatt), paket.leere, &paket.format, &paket.stil, &ordner, &fortschritt)));
            let mut j = job.lock().unwrap();
            j.laeuft = false;
            match erg {
                Ok(Ok(v)) => {
                    j.phase = "fertig".into();
                    j.ergebnis = Some(v);
                }
                Ok(Err(e)) => {
                    j.phase = "fehler".into();
                    j.fehler = Some(e);
                }
                Err(_) => {
                    j.phase = "fehler".into();
                    j.fehler = Some("Interner Fehler beim Erzeugen der Seiten".into());
                }
            }
        });
        Ok(())
    }

    pub fn starte_analyse(&self) -> Res<()> {
        let projekt = self.projekt.lock().unwrap().clone().ok_or("Zuerst einen Ordner wählen.")?;
        {
            let mut j = self.job.lock().unwrap();
            if j.laeuft {
                return Ok(());
            }
            *j = Job { laeuft: true, phase: "suchen".into(), fertig: 0, gesamt: 0, fehler: None, art: "analyse".into(), ergebnis: None };
        }
        let job = self.job.clone();
        std::thread::spawn(move || {
            let job2 = job.clone();
            let fortschritt = move |phase: &str, fertig: usize, gesamt: usize| {
                if let Ok(mut j) = job2.lock() {
                    j.phase = phase.to_string();
                    j.fertig = fertig;
                    j.gesamt = gesamt;
                }
            };
            let erg = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| analysieren(&projekt, &fortschritt)));
            let mut j = job.lock().unwrap();
            j.laeuft = false;
            match erg {
                Ok(Ok(())) => {
                    j.phase = "fertig".into();
                    if let Ok(mut p) = projekt.lock() {
                        if p.schritt < 2 {
                            p.setze_schritt(2);
                        }
                    }
                }
                Ok(Err(e)) => {
                    j.phase = "fehler".into();
                    j.fehler = Some(e);
                }
                Err(_) => {
                    j.phase = "fehler".into();
                    j.fehler = Some("Interner Fehler bei der Analyse".into());
                }
            }
        });
        Ok(())
    }
}

pub struct Antwort {
    status: u16,
    ctype: String,
    body: Vec<u8>,
    extra: Vec<(&'static str, String)>,
}

fn json_antwort(v: Value, status: u16) -> Antwort {
    Antwort {
        status,
        ctype: "application/json; charset=utf-8".into(),
        body: serde_json::to_vec(&v).unwrap(),
        extra: vec![("Cache-Control", "no-store".into())],
    }
}

fn fehler(status: u16, text: &str) -> Antwort {
    json_antwort(json!({ "fehler": text }), status)
}

fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then(|| v.to_string())
    })
}

fn konstant_gleich(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn behandle(app: &Arc<App>, req: &mut Request) -> Antwort {
    let host_ok = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Host"))
        .map(|h| {
            let v = h.value.as_str();
            let host = v.rsplit_once(':').map(|(h, _)| h).unwrap_or(v);
            host == "127.0.0.1" || host == "localhost"
        })
        .unwrap_or(false);
    if !host_ok {
        return fehler(403, "Host nicht erlaubt");
    }
    let url = req.url().to_string();
    let (pfad, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let pfad = pfad.to_string();
    let methode = req.method().clone();

    if methode == Method::Get && !pfad.starts_with("/api/") {
        return statisch(app, &pfad);
    }
    if !pfad.starts_with("/api/") {
        return fehler(403, "Kein Zugriff");
    }
    let token = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("X-Fotobuch-Token"))
        .map(|h| h.value.as_str().to_string())
        .or_else(|| query_param(query, "t"))
        .unwrap_or_default();
    if !konstant_gleich(&token, &app.token) {
        return fehler(403, "Kein Zugriff");
    }
    match methode {
        Method::Get => get(app, &pfad, query).unwrap_or_else(|e| fehler(400, &e)),
        Method::Post => {
            let ct = req.headers().iter().find(|h| h.field.equiv("Content-Type")).map(|h| h.value.as_str().to_string()).unwrap_or_default();
            if ct.split(';').next().unwrap_or("").trim() != "application/json" {
                return fehler(415, "Nur JSON erlaubt");
            }
            let mut body = Vec::new();
            if req.as_reader().take(MAX_BODY as u64 + 1).read_to_end(&mut body).is_err() || body.len() > MAX_BODY {
                return fehler(400, "Anfrage zu gross");
            }
            let d: Value = if body.is_empty() {
                json!({})
            } else {
                match serde_json::from_slice(&body) {
                    Ok(v @ Value::Object(_)) => v,
                    _ => return fehler(400, "Ungültige Anfrage"),
                }
            };
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| aktion(app, &pfad, &d)));
            match r {
                Ok(Ok(v)) => json_antwort(v, 200),
                Ok(Err(e)) => fehler(400, &e),
                Err(_) => fehler(500, "Interner Fehler"),
            }
        }
        _ => fehler(405, "Methode nicht erlaubt"),
    }
}

fn statisch(app: &App, pfad: &str) -> Antwort {
    let rel = match pfad {
        "" | "/" => "index.html",
        p => p.trim_start_matches('/'),
    };
    let Some(datei) = Ui::get(rel) else { return fehler(404, "Nicht gefunden") };
    let mut daten = datei.data.to_vec();
    if rel == "index.html" {
        daten = String::from_utf8_lossy(&daten).replace("__TOKEN__", &app.token).into_bytes();
    }
    let mut ctype = mime_guess::from_path(rel).first_or_octet_stream().essence_str().to_string();
    if ctype.starts_with("text/") || ctype == "application/javascript" || ctype == "text/javascript" {
        ctype.push_str("; charset=utf-8");
    }
    Antwort {
        status: 200,
        ctype,
        body: daten,
        extra: vec![
            ("Cache-Control", "no-store".into()),
            ("X-Content-Type-Options", "nosniff".into()),
            (
                "Content-Security-Policy",
                "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'".into(),
            ),
        ],
    }
}

fn get(app: &Arc<App>, pfad: &str, query: &str) -> Res<Antwort> {
    if pfad == "/api/status" {
        let p = app.projekt.lock().unwrap().clone();
        let ordner = p.as_ref().map(|p| p.lock().unwrap().ordner.to_string_lossy().to_string());
        let job = app.job.lock().unwrap().json();
        return Ok(json_antwort(
            json!({"version": crate::VERSION, "ordner": ordner, "job": job, "hat_projekt": p.is_some(),
                   "kann_dialog": app.ordner_dialog.is_some(), "nativ": app.nativ}),
            200,
        ));
    }
    if pfad == "/api/ansicht" {
        let p = app.aktuelles()?;
        let v = p.lock().map_err(|_| "Interner Fehler")?.ansicht();
        return Ok(json_antwort(v, 200));
    }
    if pfad == "/api/seite/layouts" {
        let key = query_param(query, "key").ok_or("Ungültige Anfrage")?;
        let p = app.aktuelles()?;
        let v = p.lock().map_err(|_| "Interner Fehler")?.layouts_fuer_seite(&key)?;
        return Ok(json_antwort(v, 200));
    }
    if let Some(id) = pfad.strip_prefix("/api/vorschau/") {
        let kante: u32 = query_param(query, "k").map(|s| s.parse().map_err(|_| "Ungültiger Wert".to_string())).transpose()?.unwrap_or(320);
        let p = app.aktuelles()?;
        let (ziel, quelle, endung, kante) = p.lock().map_err(|_| "Interner Fehler")?.vorschau_ziel(id, kante)?;
        let bytes = vorschau_bytes(&ziel, &quelle, &endung, kante);
        return Ok(Antwort { status: 200, ctype: "image/jpeg".into(), body: bytes, extra: vec![("Cache-Control", "private, max-age=3600".into())] });
    }
    Ok(fehler(404, "Unbekannt"))
}

fn text(d: &Value, k: &str) -> Res<String> {
    d.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| "Ungültige Anfrage".to_string())
}

fn aktion(app: &Arc<App>, pfad: &str, d: &Value) -> Res<Value> {
    let ok = || json!({"ok": true});
    match pfad {
        "/api/ordner/waehlen" => {
            let dlg = app.ordner_dialog.as_ref().ok_or("Auf diesem Rechner ist kein Ordner-Dialog verfügbar. Gib den Pfad ein.")?;
            match dlg() {
                None => return Ok(json!({"ok": true, "abgebrochen": true})),
                Some(g) => {
                    app.oeffne(&g)?;
                    return Ok(json!({"ok": true, "ordner": g}));
                }
            }
        }
        "/api/ordner/oeffnen" => {
            let o = d.get("pfad").and_then(|v| v.as_str()).unwrap_or("").trim().trim_matches('"').to_string();
            if o.is_empty() {
                return Err("Bitte einen Ordner angeben.".into());
            }
            app.oeffne(&o)?;
            return Ok(json!({"ok": true, "ordner": o}));
        }
        "/api/analyse/start" => {
            app.starte_analyse()?;
            return Ok(ok());
        }
        "/api/beenden" => {
            app.beenden.store(true, Ordering::SeqCst);
            return Ok(ok());
        }
        "/api/extern" => {
            let u = text(d, "url")?;
            if !(u.starts_with("https://") || u.starts_with("http://")) {
                return Err("Nur Web-Adressen erlaubt".into());
            }
            let _ = open::that(u);
            return Ok(ok());
        }
        _ => {}
    }
    let pj = app.aktuelles()?;
    let mut p = pj.lock().map_err(|_| "Interner Fehler".to_string())?;
    match pfad {
        "/api/einstellungen" => {
            p.setze_einstellungen(d.as_object().unwrap())?;
            Ok(ok())
        }
        "/api/schritt" => {
            let s = d.get("schritt").and_then(|v| v.as_i64()).ok_or("Ungültige Anfrage")?;
            p.setze_schritt(s);
            Ok(ok())
        }
        "/api/foto" => {
            p.setze_foto(&text(d, "id")?, &text(d, "aktion")?)?;
            Ok(ok())
        }
        "/api/serie" => {
            let nr = d.get("nr").and_then(|v| v.as_i64()).ok_or("Ungültige Anfrage")?;
            p.serie_entscheiden(nr, d.get("waehle").ok_or("Ungültige Anfrage")?)?;
            Ok(ok())
        }
        "/api/serien/alle-vorschlaege" => {
            if p.ergebnis.is_some() {
                p.alle_serien_auf_vorschlag();
            }
            Ok(ok())
        }
        "/api/entscheidungen/zuruecksetzen" => {
            p.alle_entscheidungen_zuruecksetzen();
            Ok(ok())
        }
        "/api/ereignis/name" => {
            p.ereignis_umbenennen(&text(d, "key")?, d.get("name").and_then(|v| v.as_str()).unwrap_or(""));
            Ok(ok())
        }
        "/api/ort/name" => {
            p.ort_umbenennen(&text(d, "key")?, d.get("name").and_then(|v| v.as_str()).unwrap_or(""));
            Ok(ok())
        }
        "/api/vorschlag/anwenden" => {
            let ids = d.get("ids").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect());
            let n = p.vorschlag_anwenden(&text(d, "art")?, ids)?;
            Ok(json!({"ok": true, "anzahl": n}))
        }
        "/api/export/seiten" => {
            let ziel = d.get("ziel").and_then(|v| v.as_str()).map(|s| s.to_string());
            drop(p);
            app.starte_seitenexport(ziel)?;
            Ok(ok())
        }
        "/api/gestaltung" => {
            p.setze_gestaltung(d.as_object().unwrap())?;
            Ok(ok())
        }
        "/api/foto/ganzseite" => {
            let art = d.get("art").and_then(|v| v.as_str());
            p.setze_seitenart(&text(d, "id")?, art)?;
            Ok(ok())
        }
        "/api/foto/seite" => {
            let seite = d.get("seite").and_then(|v| v.as_u64()).map(|n| n as usize);
            p.verschiebe_foto(&text(d, "id")?, seite)?;
            Ok(ok())
        }
        "/api/seiten/zuruecksetzen" => {
            p.verschiebungen_zuruecksetzen();
            Ok(ok())
        }
        "/api/ereignis/teilen" => {
            p.ereignis_teilen_bei(&text(d, "id")?)?;
            Ok(ok())
        }
        "/api/ereignis/zusammenlegen" => {
            p.ereignis_mit_vorherigem(&text(d, "key")?)?;
            Ok(ok())
        }
        "/api/ereignisse/zuruecksetzen" => {
            if p.ergebnis.is_some() {
                p.ereignisse_zuruecksetzen();
            }
            Ok(ok())
        }
        "/api/seite/layout" => {
            let w = d.get("wahl").and_then(|v| v.as_u64()).ok_or("Ungültige Anfrage")? as usize;
            p.setze_layout(&text(d, "key")?, w)?;
            Ok(ok())
        }
        "/api/export" => {
            if p.ergebnis.is_none() {
                return Err("Noch nicht analysiert".into());
            }
            let ziel = d.get("ziel").and_then(|v| v.as_str()).map(|s| s.to_string());
            let mut r = p.exportieren(ziel.as_deref(), true)?;
            r["ok"] = json!(true);
            Ok(r)
        }
        _ => Err("Unbekannte Aktion".into()),
    }
}

fn sende(req: Request, a: Antwort) {
    let mut r = Response::from_data(a.body).with_status_code(a.status);
    r.add_header(Header::from_bytes("Content-Type", a.ctype).unwrap());
    for (k, v) in a.extra {
        r.add_header(Header::from_bytes(k, v).unwrap());
    }
    let _ = req.respond(r);
}

/// Startet den Server in Hintergrund-Threads. Gibt die Adresse zurück (mit Port, ohne Token).
pub fn starte_server(app: Arc<App>, port: u16) -> Result<String, String> {
    let server = Server::http(("127.0.0.1", port)).map_err(|e| format!("Server konnte nicht starten: {e}"))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
    let server = Arc::new(server);
    for _ in 0..8 {
        let (server, app) = (server.clone(), app.clone());
        std::thread::spawn(move || loop {
            let Ok(mut req) = server.recv() else { break };
            let a = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| behandle(&app, &mut req))).unwrap_or_else(|_| fehler(500, "Interner Fehler"));
            sende(req, a);
        });
    }
    Ok(format!("http://127.0.0.1:{port}/"))
}
