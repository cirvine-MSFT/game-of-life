//! Integration tests for `RunSession` — the simulation state machine.
//!
//! All tests drive only the public API; per project convention there are
//! no white-box tests of `SessionData` internals.

use std::sync::Arc;

use game_of_life_desktop_lib::ipc_types::{
    CellEdit, InitialSource, IpcRunStatus, Mode, PatternName,
};
use game_of_life_desktop_lib::session::RunSession;

fn fresh_session(width: u32, height: u32, max_iter: u64) -> Arc<RunSession> {
    let s = Arc::new(RunSession::new());
    s.create_run(width, height, InitialSource::Empty, max_iter, None)
        .expect("create_run");
    s
}

#[test]
fn new_session_starts_in_setup_with_no_board() {
    let s = RunSession::new();
    let info = s.info();
    assert_eq!(info.mode, Mode::Setup);
    assert_eq!(info.iteration, 0);
    assert_eq!(info.width, 0);
    assert_eq!(info.height, 0);
    assert!(!info.dirty);
    assert!(!info.completed);
    assert!(info.save_path.is_none());
}

#[test]
fn create_run_with_empty_source_yields_a_dead_board() {
    let s = fresh_session(5, 5, 10);
    let info = s.info();
    assert_eq!(info.width, 5);
    assert_eq!(info.height, 5);
    assert_eq!(info.iteration, 0);
    assert_eq!(info.max_iterations, 10);
    let board = s.board_payload();
    assert!(!board.cells_base64.is_empty());
    let cells = board.decoded_cells().unwrap();
    assert_eq!(cells.len(), 25);
    assert!(cells.iter().all(|&c| c == 0));
}

#[test]
fn create_run_with_pattern_blinker_paints_three_alive_cells() {
    let s = Arc::new(RunSession::new());
    s.create_run(5, 5, InitialSource::Pattern(PatternName::Blinker), 10, None)
        .unwrap();
    let cells = s.board_payload().decoded_cells().unwrap();
    let alive: u64 = cells.iter().map(|&c| c as u64).sum();
    assert!(
        alive >= 3,
        "blinker must place at least 3 alive cells, got {alive}"
    );
}

#[test]
fn set_cell_paints_a_cell_and_marks_dirty() {
    let s = fresh_session(3, 3, 5);
    assert!(!s.info().dirty);
    s.set_cell(1, 1, true).unwrap();
    let info = s.info();
    assert!(info.dirty);
    let cells = s.board_payload().decoded_cells().unwrap();
    assert_eq!(cells[3 + 1], 1);
}

#[test]
fn paint_cells_applies_a_batch() {
    let s = fresh_session(4, 4, 5);
    let edits = vec![
        CellEdit {
            x: 0,
            y: 0,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 2,
            y: 2,
            alive: true,
        },
    ];
    s.paint_cells(&edits).unwrap();
    let cells = s.board_payload().decoded_cells().unwrap();
    assert_eq!(cells[0], 1);
    assert_eq!(cells[4 + 1], 1);
    assert_eq!(cells[(2 * 4) + 2], 1);
    assert_eq!(cells[(3 * 4) + 3], 0);
}

#[test]
fn clear_board_zeros_every_cell_and_marks_dirty() {
    let s = Arc::new(RunSession::new());
    s.create_run(
        3,
        3,
        InitialSource::Pattern(PatternName::FullyAlive),
        10,
        None,
    )
    .unwrap();
    let cells_before = s.board_payload().decoded_cells().unwrap();
    assert!(cells_before.contains(&1));
    s.clear_board().unwrap();
    let cells_after = s.board_payload().decoded_cells().unwrap();
    assert!(cells_after.iter().all(|&c| c == 0));
    assert!(s.info().dirty);
}

#[test]
fn randomize_is_reproducible_for_the_same_seed() {
    let s1 = fresh_session(20, 20, 10);
    s1.randomize(0xC0FFEE, 500).unwrap();
    let snap1 = s1.board_payload().decoded_cells().unwrap();

    let s2 = fresh_session(20, 20, 10);
    s2.randomize(0xC0FFEE, 500).unwrap();
    let snap2 = s2.board_payload().decoded_cells().unwrap();

    assert_eq!(snap1, snap2, "same seed must produce identical boards");
}

#[test]
fn start_run_transitions_to_paused_and_records_initial_alive_count() {
    let s = fresh_session(3, 3, 5);
    s.set_cell(1, 1, true).unwrap();
    s.start_run().unwrap();
    let info = s.info();
    assert_eq!(info.mode, Mode::Paused);
    assert_eq!(info.iteration, 0);
    let history = s.alive_history();
    assert_eq!(history, vec![1], "initial alive count must seed history");
}

#[test]
fn set_cell_in_running_mode_is_rejected() {
    let s = fresh_session(3, 3, 5);
    s.start_run().unwrap();
    let err = s.set_cell(0, 0, true).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Setup"), "got: {msg}");
}

#[test]
fn advance_one_increments_iteration_and_updates_history() {
    let s = fresh_session(3, 3, 5);
    // Blinker pattern oscillates between horizontal and vertical rows.
    s.paint_cells(&[
        CellEdit {
            x: 0,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 2,
            y: 1,
            alive: true,
        },
    ])
    .unwrap();
    s.start_run().unwrap();
    let tick = s.advance_one().unwrap();
    assert_eq!(tick.iteration, 1);
    assert_eq!(tick.alive, 3);
    let history = s.alive_history();
    assert_eq!(history.len(), 2);
}

#[test]
fn advance_one_detects_extinction_and_finalises_stats() {
    let s = fresh_session(3, 3, 10);
    // Single isolated cell dies in one generation.
    s.set_cell(1, 1, true).unwrap();
    s.start_run().unwrap();
    let tick = s.advance_one().unwrap();
    assert_eq!(tick.alive, 0);
    let info = s.info();
    assert!(info.completed);
    assert_eq!(info.status, Some(IpcRunStatus::Extinct));
    // Mode should be Paused after extinction so the user can still
    // inspect, save, or restart.
    assert_eq!(info.mode, Mode::Paused);
}

#[test]
fn advance_one_detects_stability_and_finalises_stats() {
    let s = fresh_session(2, 2, 10);
    s.paint_cells(&[
        CellEdit {
            x: 0,
            y: 0,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 0,
            alive: true,
        },
        CellEdit {
            x: 0,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
    ])
    .unwrap();
    s.start_run().unwrap();

    let tick = s.advance_one().unwrap();

    assert_eq!(tick.alive, 4);
    let info = s.info();
    assert!(info.completed);
    assert_eq!(info.iteration, 1);
    assert_eq!(info.status, Some(IpcRunStatus::Stable));
    assert_eq!(info.mode, Mode::Paused);
}

#[test]
fn advance_one_detects_max_iterations() {
    let s = fresh_session(3, 3, 2);
    s.paint_cells(&[
        CellEdit {
            x: 0,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 2,
            y: 1,
            alive: true,
        },
    ])
    .unwrap();
    s.start_run().unwrap();
    s.advance_one().unwrap();
    s.advance_one().unwrap();
    let info = s.info();
    assert!(info.completed);
    assert_eq!(info.status, Some(IpcRunStatus::MaxIterations));
}

#[test]
fn advance_after_completion_returns_error() {
    let s = fresh_session(3, 3, 1);
    s.set_cell(0, 0, true).unwrap();
    s.start_run().unwrap();
    s.advance_one().unwrap();
    let err = s.advance_one().unwrap_err();
    assert!(err.to_string().contains("completed"));
}

#[test]
fn restart_restores_initial_snapshot_and_resets_iteration() {
    let s = fresh_session(3, 3, 10);
    s.paint_cells(&[
        CellEdit {
            x: 0,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 2,
            y: 1,
            alive: true,
        },
    ])
    .unwrap();
    s.start_run().unwrap();
    s.advance_one().unwrap();
    s.advance_one().unwrap();
    s.restart().unwrap();
    let info = s.info();
    assert_eq!(info.iteration, 0);
    assert_eq!(info.mode, Mode::Paused);
    assert!(!info.completed);
    // Initial blinker (horizontal) should be restored.
    let cells = s.board_payload().decoded_cells().unwrap();
    assert_eq!(cells[3], 1);
    assert_eq!(cells[4], 1);
    assert_eq!(cells[5], 1);
}

#[test]
fn edit_board_returns_to_setup_and_drops_run_state() {
    let s = fresh_session(3, 3, 10);
    s.set_cell(0, 0, true).unwrap();
    s.start_run().unwrap();
    s.advance_one().unwrap();
    s.edit_board().unwrap();
    let info = s.info();
    assert_eq!(info.mode, Mode::Setup);
    assert_eq!(info.iteration, 0);
    assert!(!info.completed);
    // The board is intact so the user can keep painting.
    let cells = s.board_payload().decoded_cells().unwrap();
    assert_eq!(cells.len(), 9);
}

#[test]
fn extend_max_iterations_lifts_the_cap_and_clears_max_iter_status() {
    let s = fresh_session(3, 3, 1);
    s.paint_cells(&[
        CellEdit {
            x: 0,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 2,
            y: 1,
            alive: true,
        },
    ])
    .unwrap();
    s.start_run().unwrap();
    s.advance_one().unwrap();
    assert!(s.info().completed);
    s.extend_max_iterations(5).unwrap();
    let info = s.info();
    assert!(!info.completed);
    assert_eq!(info.max_iterations, 5);
}

#[test]
fn extend_max_iterations_rehydrates_stats_so_next_cap_hit_finalises() {
    // Regression for the bug where `advance_one` did `data.stats.take()`
    // at terminal state, but `extend_max_iterations` only cleared
    // `final_stats` without restoring `data.stats`. The next cap-hit
    // would then silently fail to finalise, leaving `info.completed` =
    // false forever and spinning the play worker.
    let s = fresh_session(3, 3, 1);
    s.paint_cells(&[
        CellEdit {
            x: 0,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 1,
            y: 1,
            alive: true,
        },
        CellEdit {
            x: 2,
            y: 1,
            alive: true,
        },
    ])
    .unwrap();
    s.start_run().unwrap();
    s.advance_one().unwrap();
    assert!(s.info().completed);
    s.extend_max_iterations(3).unwrap();
    assert!(!s.info().completed);
    s.advance_one().unwrap();
    s.advance_one().unwrap();
    let info = s.info();
    assert!(
        info.completed,
        "second cap hit must finalise; got {:?}",
        info
    );
    assert_eq!(info.status, Some(IpcRunStatus::MaxIterations));
}

#[test]
fn cancel_flag_round_trip() {
    let s = Arc::new(RunSession::new());
    assert!(!s.cancel_requested());
    s.request_cancel();
    assert!(s.cancel_requested());
    s.clear_cancel();
    assert!(!s.cancel_requested());
}

#[test]
fn set_cell_out_of_bounds_returns_error() {
    let s = fresh_session(3, 3, 5);
    let err = s.set_cell(10, 10, true).unwrap_err();
    assert!(err.to_string().contains("out of bounds"));
}

#[test]
fn create_run_rejects_zero_dimensions() {
    let s = RunSession::new();
    let err = s
        .create_run(0, 10, InitialSource::Empty, 10, None)
        .unwrap_err();
    assert!(err.to_string().contains("zero"));
}

#[test]
fn create_run_rejects_too_much_memory() {
    let s = RunSession::new();
    // Tiny budget so even a 100x100 board exceeds it.
    let err = s
        .create_run(100, 100, InitialSource::Empty, 10, Some(64))
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Streaming-mode boards") || msg.contains("exceeds"),
        "expected friendly streaming message, got: {msg}",
    );
}

#[test]
fn create_run_friendly_streaming_error_when_budget_exceeded() {
    let s = RunSession::new();
    // 16384 x 16384 = 256 MiB of CellState bytes, well past the 64 MiB
    // budget we pass below. The exact byte cost of CellState is 1 byte
    // (an enum with 4 variants), so size_of_val gives 1.
    let err = s
        .create_run(
            16384,
            16384,
            InitialSource::Empty,
            10,
            Some(64 * 1024 * 1024),
        )
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("16384x16384"), "got: {msg}");
    assert!(msg.contains("Streaming"), "got: {msg}");
    assert!(msg.contains("issue #10"), "got: {msg}");
}

#[test]
fn save_board_snapshot_writes_a_gol_file_and_round_trips() {
    use std::env;
    use std::fs;

    let s = fresh_session(4, 4, 10);
    s.set_cell(0, 0, true).unwrap();
    s.set_cell(1, 1, true).unwrap();
    s.set_cell(2, 2, true).unwrap();

    let tmp = env::temp_dir().join(format!("gol-desktop-test-{}.gol", std::process::id(),));
    let _ = fs::remove_file(&tmp);

    s.save_board_snapshot(&tmp).unwrap();

    let contents = fs::read_to_string(&tmp).unwrap();
    assert!(
        contents.contains("GOL-BOARD-SNAPSHOT v1"),
        "snapshot must use the standard magic header, got: {}",
        contents.lines().next().unwrap_or("")
    );

    // Refuses to overwrite without explicit removal.
    let err = s.save_board_snapshot(&tmp).unwrap_err();
    assert!(err
        .to_string()
        .to_lowercase()
        .contains("refusing to overwrite"));

    fs::remove_file(&tmp).ok();
}

#[test]
fn begin_playing_rejects_unless_paused() {
    let s = RunSession::new();
    // No board yet → Setup mode → reject.
    let err = s.begin_playing().unwrap_err();
    assert!(err.to_string().contains("Paused"));
}

#[test]
fn begin_playing_atomically_transitions_paused_to_playing() {
    let s = fresh_session(3, 3, 5);
    s.start_run().unwrap();
    assert_eq!(s.info().mode, Mode::Paused);
    s.begin_playing().unwrap();
    assert_eq!(s.info().mode, Mode::Playing);
    // Second call must fail since we're no longer Paused.
    let err = s.begin_playing().unwrap_err();
    assert!(err.to_string().contains("Paused"));
}

#[test]
fn begin_jumping_atomically_transitions_paused_to_jumping_to() {
    let s = fresh_session(3, 3, 100);
    s.start_run().unwrap();
    s.begin_jumping(50).unwrap();
    let info = s.info();
    assert_eq!(info.mode, Mode::JumpingTo);
    assert_eq!(info.jump_target, Some(50));
}
