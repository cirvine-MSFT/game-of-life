//! Integration tests for the IPC wire-format types.
//!
//! These are integration tests per the project convention; only the public
//! API of `game_of_life_desktop_lib::ipc_types` is exercised.

use game_of_life::stats::run_statistics::{CycleStatistics, RunStatus};
use game_of_life::{AdvanceOutcome, CellState, RunStatistics};
use game_of_life_desktop_lib::ipc_types::{
    AdvanceTick, BoardPayload, CellEdit, InitialSource, IpcCellState, IpcRunStatistics,
    IpcRunStatus, Mode, PatternName, SessionInfo,
};

#[test]
fn ipc_cell_state_collapses_transitional_variants_to_alive_or_dead() {
    assert_eq!(IpcCellState::from_core(CellState::Dead), IpcCellState::Dead);
    assert_eq!(
        IpcCellState::from_core(CellState::Alive),
        IpcCellState::Alive
    );
    // Transitional states are renderer-internal; once advance_generation
    // returns, only Dead/Alive remain, but we still want defined behaviour
    // if a snapshot ever carries one through.
    assert_eq!(
        IpcCellState::from_core(CellState::Dying),
        IpcCellState::Dead
    );
    assert_eq!(
        IpcCellState::from_core(CellState::Resurrecting),
        IpcCellState::Alive,
    );
}

#[test]
fn ipc_cell_state_round_trips_normalized_core_state() {
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

#[test]
fn board_payload_round_trips_through_base64() {
    let cells: Vec<u8> = (0..2500)
        .map(|i| if i % 3 == 0 { 1u8 } else { 0u8 })
        .collect();
    let payload = BoardPayload::from_bytes(50, 50, 12, &cells);
    let decoded = payload.decoded_cells().expect("base64 must round-trip");
    assert_eq!(decoded, cells);
    assert_eq!(payload.width, 50);
    assert_eq!(payload.height, 50);
    assert_eq!(payload.iteration, 12);
}

#[test]
fn advance_tick_computes_dead_from_total_minus_alive() {
    let outcome = AdvanceOutcome::from_counts(7, 3, 200);
    let tick = AdvanceTick::from_outcome(42, 25 * 25, outcome);
    assert_eq!(tick.iteration, 42);
    assert_eq!(tick.alive, 200);
    assert_eq!(tick.dead, 25 * 25 - 200);
    assert_eq!(tick.births, 7);
    assert_eq!(tick.deaths, 3);
}

#[test]
fn run_statistics_conversion_preserves_every_field() {
    let s = RunStatistics {
        initial_alive_count: 10,
        final_alive_count: 4,
        peak_alive_count: 18,
        peak_alive_generation: 6,
        min_alive_count: 1,
        min_alive_generation: 20,
        total_births: 100,
        total_deaths: 106,
        iterations_run: 25,
        status: RunStatus::Extinct,
        cycle: None,
    };
    let ipc = IpcRunStatistics::from(&s);
    assert_eq!(ipc.initial_alive_count, 10);
    assert_eq!(ipc.final_alive_count, 4);
    assert_eq!(ipc.peak_alive_count, 18);
    assert_eq!(ipc.peak_alive_generation, 6);
    assert_eq!(ipc.min_alive_count, 1);
    assert_eq!(ipc.min_alive_generation, 20);
    assert_eq!(ipc.total_births, 100);
    assert_eq!(ipc.total_deaths, 106);
    assert_eq!(ipc.iterations_run, 25);
    assert_eq!(ipc.status, IpcRunStatus::Extinct);
    assert_eq!(ipc.cycle_start_generation, None);
    assert_eq!(ipc.cycle_detected_generation, None);
    assert_eq!(ipc.cycle_period, None);
}

#[test]
fn run_statistics_conversion_preserves_cycle_metadata() {
    let s = RunStatistics {
        initial_alive_count: 3,
        final_alive_count: 3,
        peak_alive_count: 3,
        peak_alive_generation: 0,
        min_alive_count: 3,
        min_alive_generation: 0,
        total_births: 6,
        total_deaths: 6,
        iterations_run: 2,
        status: RunStatus::Cyclic,
        cycle: Some(CycleStatistics {
            start_generation: 0,
            detected_generation: 2,
            period: 2,
        }),
    };

    let ipc = IpcRunStatistics::from(&s);

    assert_eq!(ipc.cycle_start_generation, Some(0));
    assert_eq!(ipc.cycle_detected_generation, Some(2));
    assert_eq!(ipc.cycle_period, Some(2));
}

#[test]
fn stable_run_status_converts_to_ipc_status() {
    assert_eq!(
        IpcRunStatus::from_core(RunStatus::Stable),
        IpcRunStatus::Stable
    );
}

#[test]
fn cyclic_run_status_converts_to_ipc_status() {
    assert_eq!(
        IpcRunStatus::from_core(RunStatus::Cyclic),
        IpcRunStatus::Cyclic
    );
}

#[test]
fn session_info_round_trips_through_json() {
    let info = SessionInfo {
        mode: Mode::Playing,
        iteration: 42,
        width: 32,
        height: 32,
        max_iterations: 100,
        save_path: Some("/tmp/run.gol".into()),
        dirty: true,
        completed: false,
        jump_target: None,
        status: None,
    };
    let json = serde_json::to_string(&info).unwrap();
    let decoded: SessionInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, info);
}

#[test]
fn initial_source_serialises_with_tag_and_value() {
    let json = serde_json::to_string(&InitialSource::Pattern(PatternName::Blinker)).unwrap();
    assert!(json.contains("\"kind\":\"pattern\""), "got: {json}");
    assert!(json.contains("\"value\":\"blinker\""), "got: {json}");

    let json = serde_json::to_string(&InitialSource::Random {
        seed: 17,
        alive_cells_per_thousand: 400,
    })
    .unwrap();
    assert!(json.contains("\"kind\":\"random\""), "got: {json}");
    assert!(json.contains("\"seed\":17"), "got: {json}");
}

#[test]
fn cell_edit_round_trips_through_serde() {
    let edits = vec![
        CellEdit {
            x: 1,
            y: 2,
            alive: true,
        },
        CellEdit {
            x: 3,
            y: 4,
            alive: false,
        },
    ];
    let json = serde_json::to_string(&edits).unwrap();
    let decoded: Vec<CellEdit> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, edits);
}
