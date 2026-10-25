//! Entwickler-Werkzeug ohne Fenster: `fotobuch-dev serve [ordner] [port]` startet nur den Server,
//! `fotobuch-dev dump <ordner>` analysiert und gibt die Ansicht als JSON aus.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;

use fotobuch_core::projekt::{analysieren, Projekt};
use fotobuch_core::server::{starte_server, App};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("dump") => {
            let p = Arc::new(Mutex::new(Projekt::neu(&PathBuf::from(&a[2])).expect("Ordner")));
            analysieren(&p, &|_, _, _| {}).expect("Analyse");
            println!("{}", serde_json::to_string(&p.lock().unwrap().ansicht()).unwrap());
        }
        Some("serve") => {
            let app = App::neu(None, false);
            if let Some(o) = a.get(2).filter(|o| !o.is_empty()) {
                app.oeffne(o).expect("Ordner");
            }
            let port: u16 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let url = starte_server(app.clone(), port).expect("Server");
            println!("{url}");
            while !app.beenden.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        _ => eprintln!("Aufruf: fotobuch-dev serve [ordner] [port] | dump <ordner>"),
    }
}
