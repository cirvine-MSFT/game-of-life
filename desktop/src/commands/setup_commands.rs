//! Setup-phase commands: create a run, paint cells, apply patterns,
//! randomize, clear. Every command here is rejected by the session
//! when called outside Setup mode.

use std::sync::Arc;

use tauri::State;

use crate::ipc_types::{CellEdit, InitialSource, PatternName};
use crate::session::{RunSession, SessionError};

#[tauri::command]
pub fn create_run(
    session: State<'_, Arc<RunSession>>,
    width: u32,
    height: u32,
    source: InitialSource,
    max_iterations: u64,
    max_memory_bytes: Option<usize>,
) -> Result<(), SessionError> {
    session.create_run(width, height, source, max_iterations, max_memory_bytes)
}

#[tauri::command]
pub fn set_cell(
    session: State<'_, Arc<RunSession>>,
    x: u32,
    y: u32,
    alive: bool,
) -> Result<(), SessionError> {
    session.set_cell(x, y, alive)
}

#[tauri::command]
pub fn paint_cells(
    session: State<'_, Arc<RunSession>>,
    edits: Vec<CellEdit>,
) -> Result<(), SessionError> {
    session.paint_cells(&edits)
}

#[tauri::command]
pub fn apply_pattern(
    session: State<'_, Arc<RunSession>>,
    pattern: PatternName,
) -> Result<(), SessionError> {
    session.apply_pattern(pattern)
}

#[tauri::command]
pub fn randomize(
    session: State<'_, Arc<RunSession>>,
    seed: u64,
    alive_cells_per_thousand: u16,
) -> Result<(), SessionError> {
    session.randomize(seed, alive_cells_per_thousand)
}

#[tauri::command]
pub fn clear_board(session: State<'_, Arc<RunSession>>) -> Result<(), SessionError> {
    session.clear_board()
}
