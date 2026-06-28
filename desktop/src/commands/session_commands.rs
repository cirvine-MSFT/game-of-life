//! Read-only session queries plus path helpers.
//!
//! These commands never mutate session state, so they can run during
//! any mode without serialising against the play/jump worker. The one
//! exception is `save_board_snapshot`, which reads the current board
//! and writes it to disk — kept here because it shares the read-only
//! "I don't care what mode you're in" semantics.

use std::path::PathBuf;
use std::sync::Arc;

use game_of_life::persistence::{
    read_run_record, sniff_file_kind, ContentHashMode, FileKind, DEFAULT_MAX_INPUT_FILE_BYTES,
};
use tauri::State;

use crate::ipc_types::{
    BoardPayload, IpcIterationSeries, IpcRunSeries, IpcRunStatistics, RunBoardSelection,
    SessionInfo,
};
use crate::session::{RunSession, SessionError, DEFAULT_MAX_BOARD_MEMORY_BYTES};

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

#[tauri::command]
pub fn read_run_series(path: String) -> Result<IpcRunSeries, String> {
    let p = PathBuf::from(&path);
    let kind = sniff_file_kind(&p).map_err(|e| e.to_string())?;
    if kind != FileKind::RunRecord {
        return Err(format!(
            "File '{}' is a {}, but expected a run record.",
            p.display(),
            kind
        ));
    }

    let record = read_run_record(
        &p,
        DEFAULT_MAX_BOARD_MEMORY_BYTES,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .map_err(|e| e.to_string())?;
    let filename = p
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());

    Ok(IpcRunSeries {
        path,
        filename,
        summary: IpcRunStatistics::try_from_result(&record.result)?,
        series: record.series.as_ref().map(IpcIterationSeries::from),
    })
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

/// Saves the current board as a standalone `GOL-BOARD-SNAPSHOT v1`
/// file. Mirrors the CLI's `--save-board` flag so users have an
/// escape hatch for "this board is interesting, preserve it" without
/// needing the full run record (which can only be saved post-run).
///
/// `overwrite=true` removes any pre-existing file at `path` before
/// writing — gives the frontend a clean "Save As" UX where the
/// confirmation dialog gates the destructive step.
#[tauri::command]
pub fn save_board_snapshot(
    session: State<'_, Arc<RunSession>>,
    path: String,
    overwrite: bool,
) -> Result<String, SessionError> {
    let p = PathBuf::from(&path);
    if overwrite && p.exists() {
        std::fs::remove_file(&p).map_err(|e| {
            SessionError::SaveBoardSnapshot(format!("Failed to overwrite {}: {}", p.display(), e))
        })?;
    }
    session.save_board_snapshot(&p)?;
    Ok(p.display().to_string())
}

#[tauri::command]
pub fn load_board_snapshot(
    session: State<'_, Arc<RunSession>>,
    path: String,
) -> Result<String, SessionError> {
    let p = PathBuf::from(&path);
    session.load_board_snapshot(&p)?;
    Ok(p.display().to_string())
}

#[tauri::command]
pub fn load_run_board(
    session: State<'_, Arc<RunSession>>,
    path: String,
    selection: RunBoardSelection,
) -> Result<String, SessionError> {
    let p = PathBuf::from(&path);
    session.load_run_board(&p, selection)?;
    Ok(p.display().to_string())
}
