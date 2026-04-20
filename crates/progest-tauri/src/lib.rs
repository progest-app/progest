//! Tauri IPC shell for Progest.
//!
//! This crate is intentionally thin: it wires `progest-core` APIs to
//! Tauri commands that the React frontend calls. Business logic does
//! not live here.

/// Initializes logging and runs the Tauri application.
///
/// # Panics
///
/// Panics if the Tauri runtime fails to build or run. Tauri's own error
/// reporting is surfaced before the panic.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
