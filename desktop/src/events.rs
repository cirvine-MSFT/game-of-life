//! Names of events emitted from the Rust side to the frontend.
//!
//! Centralising them here keeps the Rust event-emission and the
//! frontend's `listen()` calls in lock-step. The frontend mirrors these
//! string constants in `desktop/ui/src/ipc/events.ts`.

pub const BOARD_TICK: &str = "gol://board-tick";
pub const JUMP_PROGRESS: &str = "gol://jump-progress";
pub const RUN_COMPLETED: &str = "gol://run-completed";
pub const SESSION_CHANGED: &str = "gol://session-changed";
pub const ERROR: &str = "gol://error";
