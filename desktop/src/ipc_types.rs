//! Wire-format types for IPC between the Tauri Rust side and the React
//! frontend.
//!
//! The core `game-of-life` crate has zero `serde` derives by design — its
//! public types stay free of any I/O concern. The desktop crate therefore
//! owns its own wire layer: serde-derived shapes that mirror what the
//! frontend actually needs to render, plus `From`/conversion helpers that
//! translate to and from the core types.
//!
//! Field naming is camelCase across the wire to match JavaScript
//! conventions on the frontend.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};

use game_of_life::stats::run_statistics::RunStatus;
use game_of_life::{AdvanceOutcome, CellState, RunStatistics};

/// Top-level mode the frontend reflects in its UI (toolbar badge, edit
/// menu enablement, lock cursor on the canvas, etc.).
///
/// `JumpingTo` is reported while a Run-to-N or Jump-to-N worker is racing
/// in the background; the target iteration is published separately so the
/// frontend can render a progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Setup,
    Paused,
    Playing,
    JumpingTo,
}

/// Cell colour as it crosses the wire.
///
/// Core uses four-variant `CellState` (Dead/Alive/Dying/Resurrecting) but
/// the transitional variants exist only inside the updater's mark phase;
/// after `advance_generation` returns, the board contains only Dead/Alive.
/// The animation in the canvas is computed from a desktop-side
/// pre/post diff, not from this enum, so two variants are enough here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IpcCellState {
    Dead,
    Alive,
}

impl IpcCellState {
    pub fn from_core(state: CellState) -> Self {
        match state {
            CellState::Alive | CellState::Resurrecting => IpcCellState::Alive,
            CellState::Dead | CellState::Dying => IpcCellState::Dead,
        }
    }

    pub fn to_core(self) -> CellState {
        match self {
            IpcCellState::Alive => CellState::Alive,
            IpcCellState::Dead => CellState::Dead,
        }
    }
}

/// Built-in patterns surfaced in the New Run dialog.
///
/// `CenteredBlinker` is deliberately omitted — it has no matching label in
/// the CLI's `InitialBoardSource` parser, so a run started from it would
/// not round-trip cleanly through `.gol` provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternName {
    Demo,
    Blinker,
    FullyAlive,
}

/// How a freshly created run gets its starting board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum InitialSource {
    Pattern(PatternName),
    Random {
        seed: u64,
        alive_cells_per_thousand: u16,
    },
    Empty,
}

/// Snapshot of cell bytes shipped to the frontend.
///
/// Cells are packed one byte per cell (Dead=0, Alive=1) and base64-encoded
/// so the JSON-over-IPC bridge stays small. A 1000x1000 board encodes to
/// roughly 1.3 MB of base64 — order of magnitude smaller than the JSON
/// number-array alternative and easy to decode in the frontend via
/// `atob`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardPayload {
    pub width: u32,
    pub height: u32,
    pub iteration: u64,
    pub cells_base64: String,
}

impl BoardPayload {
    pub fn from_bytes(width: u32, height: u32, iteration: u64, cells: &[u8]) -> Self {
        Self {
            width,
            height,
            iteration,
            cells_base64: BASE64.encode(cells),
        }
    }

    pub fn decoded_cells(&self) -> Result<Vec<u8>, base64::DecodeError> {
        BASE64.decode(self.cells_base64.as_bytes())
    }
}

/// Per-generation outcome plus iteration index, emitted as a `board-tick`
/// event after each advance.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceTick {
    pub iteration: u64,
    pub alive: u64,
    pub dead: u64,
    pub births: u64,
    pub deaths: u64,
}

impl AdvanceTick {
    pub fn from_outcome(iteration: u64, total_cells: u64, outcome: AdvanceOutcome) -> Self {
        Self {
            iteration,
            alive: outcome.alive_count,
            dead: total_cells.saturating_sub(outcome.alive_count),
            births: outcome.births,
            deaths: outcome.deaths,
        }
    }
}

/// Sent piggy-backed on each `board-tick` so the frontend can update its
/// canvas without a follow-up `get_board` round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardTick {
    #[serde(flatten)]
    pub stats: AdvanceTick,
    pub board: BoardPayload,
}

/// Progress emitted while a Jump-to-N or Run-to-N worker is in flight.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JumpProgress {
    pub current: u64,
    pub target: u64,
}

/// Terminal-state notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunCompleted {
    pub iteration: u64,
    pub status: IpcRunStatus,
    pub stats: IpcRunStatistics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IpcRunStatus {
    MaxIterations,
    Extinct,
    Stable,
    Cyclic,
}

impl IpcRunStatus {
    pub fn from_core(status: RunStatus) -> Self {
        match status {
            RunStatus::MaxIterations => IpcRunStatus::MaxIterations,
            RunStatus::Extinct => IpcRunStatus::Extinct,
            RunStatus::Stable => IpcRunStatus::Stable,
            RunStatus::Cyclic => IpcRunStatus::Cyclic,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IpcRunStatistics {
    pub initial_alive_count: u64,
    pub final_alive_count: u64,
    pub peak_alive_count: u64,
    pub peak_alive_generation: u64,
    pub min_alive_count: u64,
    pub min_alive_generation: u64,
    pub total_births: u64,
    pub total_deaths: u64,
    pub iterations_run: u64,
    pub status: IpcRunStatus,
    pub cycle_start_generation: Option<u64>,
    pub cycle_detected_generation: Option<u64>,
    pub cycle_period: Option<u64>,
}

impl From<&RunStatistics> for IpcRunStatistics {
    fn from(s: &RunStatistics) -> Self {
        Self {
            initial_alive_count: s.initial_alive_count,
            final_alive_count: s.final_alive_count,
            peak_alive_count: s.peak_alive_count,
            peak_alive_generation: s.peak_alive_generation,
            min_alive_count: s.min_alive_count,
            min_alive_generation: s.min_alive_generation,
            total_births: s.total_births,
            total_deaths: s.total_deaths,
            iterations_run: s.iterations_run,
            status: IpcRunStatus::from_core(s.status),
            cycle_start_generation: s.cycle.map(|cycle| cycle.start_generation),
            cycle_detected_generation: s.cycle.map(|cycle| cycle.detected_generation),
            cycle_period: s.cycle.map(|cycle| cycle.period),
        }
    }
}

/// What `get_session` returns. Drives the toolbar mode badge, the menu
/// enablement state, and the close-with-unsaved-changes guard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub mode: Mode,
    pub iteration: u64,
    pub width: u32,
    pub height: u32,
    pub max_iterations: u64,
    pub save_path: Option<String>,
    pub dirty: bool,
    pub completed: bool,
    pub jump_target: Option<u64>,
    pub status: Option<IpcRunStatus>,
}

/// A single cell paint instruction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CellEdit {
    pub x: u32,
    pub y: u32,
    pub alive: bool,
}

#[cfg(test)]
mod inline_smoke {
    // Only minimal #[cfg(test)] sanity checks — comprehensive coverage lives
    // in `tests/ipc_types.rs` per the project's integration-test convention.
    use super::*;

    #[test]
    fn cell_state_round_trip() {
        for state in [
            CellState::Dead,
            CellState::Alive,
            CellState::Dying,
            CellState::Resurrecting,
        ] {
            let ipc = IpcCellState::from_core(state);
            let back = ipc.to_core();
            match state {
                CellState::Alive | CellState::Resurrecting => assert_eq!(back, CellState::Alive),
                CellState::Dead | CellState::Dying => assert_eq!(back, CellState::Dead),
            }
        }
    }
}
