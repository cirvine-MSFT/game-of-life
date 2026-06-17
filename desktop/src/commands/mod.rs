//! Tauri `#[tauri::command]` surface. Each submodule groups one phase
//! of the workflow so the file map matches the user-facing UX:
//!
//! - `session_commands` — read-only and metadata queries.
//! - `setup_commands` — Setup-phase board editing.
//! - `run_commands` — Running-phase playback (start, play, step, jump,
//!   restart, pause, edit_board, extend).
//!
//! File I/O (`load_run_file`, `save_run`, etc.) lives in `io_commands`
//! and is wired in by the `file-ops` todo.

pub mod run_commands;
pub mod session_commands;
pub mod setup_commands;
