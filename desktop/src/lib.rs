//! Game of Life desktop app library entry point.
//!
//! Splitting `run()` into a library function keeps the binary stub minimal
//! and lets future integration tests (or mobile parity) drive the same
//! Tauri builder.

pub mod commands;
pub mod events;
pub mod ipc_types;
pub mod session;

pub fn run() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(session::RunSession::new()))
        .invoke_handler(tauri::generate_handler![
            commands::session_commands::get_session,
            commands::session_commands::get_board,
            commands::session_commands::get_alive_history,
            commands::session_commands::get_final_stats,
            commands::session_commands::default_save_dir,
            commands::setup_commands::create_run,
            commands::setup_commands::set_cell,
            commands::setup_commands::paint_cells,
            commands::setup_commands::apply_pattern,
            commands::setup_commands::randomize,
            commands::setup_commands::clear_board,
            commands::run_commands::start_run,
            commands::run_commands::restart,
            commands::run_commands::edit_board,
            commands::run_commands::extend_max_iterations,
            commands::run_commands::step,
            commands::run_commands::pause,
            commands::run_commands::play,
            commands::run_commands::jump_to,
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
