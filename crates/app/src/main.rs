// Unter Windows kein Konsolenfenster
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::Ordering;
use std::time::Duration;

use fotobuch_core::server::{starte_server, App};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|tauri_app| {
            let handle = tauri_app.handle().clone();
            let dlg_handle = handle.clone();
            let dialog: fotobuch_core::server::OrdnerDialog = Box::new(move || {
                dlg_handle
                    .dialog()
                    .file()
                    .set_title("Ordner mit den Fotos wählen")
                    .blocking_pick_folder()
                    .and_then(|p| p.into_path().ok())
                    .map(|p| p.to_string_lossy().to_string())
            });
            let app = App::neu(Some(dialog), true);
            // Selbsttest-Hilfen für die Entwicklung: FOTOBUCH_ORDNER öffnet direkt einen Ordner.
            if let Ok(o) = std::env::var("FOTOBUCH_ORDNER") {
                let _ = app.oeffne(&o);
            }
            let adresse = starte_server(app.clone(), 0).map_err(|e| e.to_string())?;
            let url = format!("{}?t={}", adresse, app.token);
            WebviewWindowBuilder::new(tauri_app, "main", WebviewUrl::External(url.parse().unwrap()))
                .title("Fotobuch-Assistent")
                .inner_size(1280.0, 860.0)
                .min_inner_size(900.0, 600.0)
                .build()?;
            std::thread::spawn(move || loop {
                if app.beenden.load(Ordering::SeqCst) {
                    handle.exit(0);
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            });
            let _ = tauri_app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Fehler beim Start der App");
}
