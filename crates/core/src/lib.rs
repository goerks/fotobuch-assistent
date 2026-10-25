//! Kern des Fotobuch-Assistenten. Alles läuft lokal, ohne Netzwerk und ohne KI-Modell.

pub mod duplicates;
pub mod events;
pub mod foto;
pub mod orte;
pub mod pages;
pub mod projekt;
pub mod scanner;
pub mod server;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
