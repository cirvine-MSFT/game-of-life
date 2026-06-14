//! Read-only session queries plus path helpers.
//!
//! These commands never mutate session state, so they can run during
//! any mode without serialising against the play/jump worker.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::ipc_types::{BoardPayload, IpcRunStatistics, SessionInfo};
use crate::session::RunSession;

#[tauri::command]
pub fn get_session(session: State<'_, Arc<RunSession>>) -> SessionInfo {
    session.info()
}

#[tauri::command]
pub fn get_board(session: State<'_, Arc<RunSession>>) -> BoardPayload {
    session.board_payload()
}

#[tauri::command]
pub fn get_alive_history(session: State<'_, Arc<RunSession>>) -> Vec<u64> {
    session.alive_history()
}

#[tauri::command]
pub fn get_final_stats(session: State<'_, Arc<RunSession>>) -> Option<IpcRunStatistics> {
    session.final_stats()
}

/// Returns the platform-appropriate default directory for saving .gol
/// files. Prefers `<Documents>/Game of Life/runs/`; falls back to the
/// per-user data dir if Documents is unavailable (rare on Linux without
/// xdg-user-dirs installed).
///
/// The directory is **created if missing** so the frontend can hand the
/// path straight to a Save dialog without first probing existence.
#[tauri::command]
pub fn default_save_dir(app: tauri::AppHandle) -> Result<String, String> {
    let path = default_save_dir_path(&app).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&path).map_err(|e| {
        format!(
            "Failed to create default save directory at {}: {}",
            path.display(),
            e
        )
    })?;
    Ok(path.display().to_string())
}

fn default_save_dir_path(app: &tauri::AppHandle) -> Result<PathBuf, tauri::Error> {
    use tauri::Manager;

    if let Ok(docs) = app.path().document_dir() {
        return Ok(docs.join("Game of Life").join("runs"));
    }
    let data = app.path().app_data_dir()?;
    Ok(data.join("runs"))
}
