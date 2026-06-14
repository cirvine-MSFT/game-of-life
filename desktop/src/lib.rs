//! Game of Life desktop app library entry point.
//!
//! Splitting `run()` into a library function keeps the binary stub minimal
//! and lets future integration tests (or mobile parity) drive the same
//! Tauri builder.

pub fn run() {
    tauri::Builder::default()
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
