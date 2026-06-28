use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use game_of_life::persistence::{
    board_grid_hash, write_board_snapshot, write_run_record, BoardSnapshot, RunId, RunRecord,
    RunRecordConfig, RunRecordResult, RUN_RECORD_SCHEMA_VERSION,
};
use game_of_life::{
    BoardEditor, BoardSize, CellCoordinate, CellState, InMemoryBoard, IterationSeries,
};
use game_of_life_desktop_lib::commands::session_commands::read_run_series;
use game_of_life_desktop_lib::ipc_types::IpcRunStatus;

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_dir(label: &str) -> PathBuf {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-output")
        .join(format!(
            "gol_aggregate_series_{label}_{}_{seq}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn board_from_grid(lines: &[&str]) -> InMemoryBoard {
    let height = lines.len();
    let width = lines.first().map(|line| line.len()).unwrap_or(0);
    let mut board = InMemoryBoard::new(width, height);
    for (y, row) in lines.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let state = if ch == '#' {
                CellState::Alive
            } else {
                CellState::Dead
            };
            board
                .set_cell(CellCoordinate::new(x, y), state)
                .expect("grid coordinates are in bounds");
        }
    }
    board
}

fn fixture_series() -> IterationSeries {
    IterationSeries {
        alive: vec![1, 3, 2],
        births: vec![0, 2, 0],
        deaths: vec![0, 0, 1],
    }
}

fn fixture_record(series: Option<IterationSeries>) -> RunRecord {
    let initial_board = board_from_grid(&["#..", "...", "..."]);
    let final_board = board_from_grid(&["##.", "...", "..."]);
    RunRecord {
        run_id: RunId::from_bytes([0x33; 16]),
        schema_version: RUN_RECORD_SCHEMA_VERSION,
        created_at: UNIX_EPOCH + Duration::from_secs(1_780_000_123),
        tool_version: "test".to_string(),
        config: RunRecordConfig {
            board_size: BoardSize::new(3, 3).unwrap(),
            max_iterations: 2,
            max_board_memory_bytes: 64 * 1024 * 1024,
            initial_board_source: "test".to_string(),
            random_seed: 0,
            updater: "in_place_transitional".to_string(),
            continued_from: None,
        },
        result: RunRecordResult {
            status: "max_iterations".to_string(),
            iterations_run: 2,
            wall_time_ms: 9,
            initial_alive_count: 1,
            final_alive_count: 2,
            peak_alive_count: 3,
            peak_alive_generation: 1,
            min_alive_count: 1,
            min_alive_generation: 0,
            total_births: 2,
            total_deaths: 1,
            cycle_start_generation: None,
            cycle_detected_generation: None,
            cycle_period: None,
            initial_board_hash: board_grid_hash(&initial_board),
            final_board_hash: board_grid_hash(&final_board),
        },
        series,
        initial_board,
        final_board,
    }
}

#[test]
fn read_run_series_returns_series_for_v2_file_with_series() {
    let dir = unique_dir("v2_series");
    let path = dir.join("run.gol");
    write_run_record(&path, &fixture_record(Some(fixture_series()))).unwrap();

    let loaded = read_run_series(path.display().to_string()).unwrap();

    assert_eq!(loaded.path, path.display().to_string());
    assert_eq!(loaded.filename, "run.gol");
    assert_eq!(loaded.summary.status, IpcRunStatus::MaxIterations);
    assert_eq!(loaded.summary.iterations_run, 2);
    assert_eq!(loaded.summary.initial_alive_count, 1);
    assert_eq!(loaded.summary.final_alive_count, 2);
    let series = loaded.series.expect("v2 fixture includes series");
    assert_eq!(series.alive, vec![1, 3, 2]);
    assert_eq!(series.births, vec![0, 2, 0]);
    assert_eq!(series.deaths, vec![0, 0, 1]);
}

#[test]
fn read_run_series_returns_none_series_for_v1_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("demo-runs")
        .join("block-2x2-stable.gol");

    let loaded = read_run_series(path.display().to_string()).unwrap();

    assert_eq!(loaded.filename, "block-2x2-stable.gol");
    assert_eq!(loaded.series, None);
    assert_eq!(loaded.summary.status, IpcRunStatus::Stable);
    assert_eq!(loaded.summary.iterations_run, 0);
    assert_eq!(loaded.summary.initial_alive_count, 4);
    assert_eq!(loaded.summary.final_alive_count, 4);
}

#[test]
fn negative_read_run_series_missing_file_returns_error() {
    let dir = unique_dir("missing");
    let path = dir.join("missing.gol");

    let err = read_run_series(path.display().to_string()).unwrap_err();

    assert!(err.contains(&path.display().to_string()), "{err}");
}

#[test]
fn negative_read_run_series_on_board_snapshot_returns_error() {
    let dir = unique_dir("snapshot");
    let path = dir.join("board.gol");
    write_board_snapshot(
        &path,
        &BoardSnapshot::for_board(board_from_grid(&["##", "##"])),
    )
    .unwrap();

    let err = read_run_series(path.display().to_string()).unwrap_err();

    assert!(err.contains("board snapshot"), "{err}");
    assert!(err.contains("run record"), "{err}");
}
