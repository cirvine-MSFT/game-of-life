//! Run-record v2 per-iteration series coverage.

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use game_of_life::persistence::{
    board_grid_hash, read_run_record, sniff_from_reader, write_run_record, ContentHashMode,
    FileKind, MagicError, ParseError, RunId, RunRecord, RunRecordConfig, RunRecordReadError,
    RunRecordResult, DEFAULT_MAX_INPUT_FILE_BYTES, RUN_RECORD_MAGIC_V2, RUN_RECORD_SCHEMA_VERSION,
};
use game_of_life::{BoardSize, CellState, InMemoryBoard, IterationSeries};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_dir(label: &str) -> PathBuf {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-output")
        .join(format!(
            "gol_iteration_series_{label}_{}_{seq}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn board(alive_at: &[(usize, usize)]) -> InMemoryBoard {
    let mut board = InMemoryBoard::new(2, 2);
    for &(x, y) in alive_at {
        board.set(x, y, CellState::Alive);
    }
    board
}

fn fixture_record(series: Option<IterationSeries>) -> RunRecord {
    let initial_board = board(&[(0, 0)]);
    let final_board = board(&[(0, 0), (1, 1)]);
    RunRecord {
        run_id: RunId::from_bytes([0x42; 16]),
        schema_version: RUN_RECORD_SCHEMA_VERSION,
        created_at: UNIX_EPOCH + Duration::from_secs(1_780_000_001),
        tool_version: "test".to_string(),
        config: RunRecordConfig {
            board_size: BoardSize::new(2, 2).unwrap(),
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
            wall_time_ms: 7,
            initial_alive_count: 1,
            final_alive_count: 2,
            peak_alive_count: 2,
            peak_alive_generation: 1,
            min_alive_count: 1,
            min_alive_generation: 0,
            total_births: 1,
            total_deaths: 0,
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

fn fixture_series() -> IterationSeries {
    IterationSeries {
        alive: vec![1, 2, 2],
        births: vec![0, 1, 0],
        deaths: vec![0, 0, 0],
    }
}

#[test]
fn roundtrip_v2_with_series_byte_equal() {
    let dir = unique_dir("with_series");
    let first = dir.join("first.gol");
    let second = dir.join("second.gol");
    let original = fixture_record(Some(fixture_series()));

    write_run_record(&first, &original).unwrap();
    let loaded = read_run_record(
        &first,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();
    assert_eq!(loaded.run_id, original.run_id);
    assert_eq!(loaded.schema_version, RUN_RECORD_SCHEMA_VERSION);
    assert_eq!(loaded.result.iterations_run, original.result.iterations_run);
    assert_eq!(loaded.series, original.series);
    assert_eq!(loaded.initial_board, original.initial_board);
    assert_eq!(loaded.final_board, original.final_board);

    write_run_record(&second, &loaded).unwrap();
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap()
    );
}

#[test]
fn roundtrip_v2_without_series() {
    let dir = unique_dir("without_series");
    let path = dir.join("run.gol");
    let original = fixture_record(None);

    write_run_record(&path, &original).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with(RUN_RECORD_MAGIC_V2));
    assert!(body.contains("schema_version: 2"));
    assert!(!body.contains("[series]"));

    let loaded = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();
    assert_eq!(loaded.series, None);
    assert_eq!(loaded.schema_version, RUN_RECORD_SCHEMA_VERSION);
}

#[test]
fn negative_v2_series_length_mismatch_rejected() {
    let dir = unique_dir("length_mismatch");
    let path = dir.join("run.gol");
    write_run_record(&path, &fixture_record(Some(fixture_series()))).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, body.replacen("alive: 1,2,2", "alive: 1,2", 1)).unwrap();

    let err = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Ignore,
    )
    .unwrap_err();
    match err {
        RunRecordReadError::Parse(ParseError::SeriesLengthMismatch {
            field,
            expected_len,
            actual_len,
            ..
        }) => {
            assert_eq!(field, "alive");
            assert_eq!(expected_len, 3);
            assert_eq!(actual_len, 2);
        }
        other => panic!("expected series length mismatch; got {other:?}"),
    }
}

#[test]
fn edge_case_load_v1_demo_file_yields_no_series() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("demo-runs")
        .join("block-2x2-stable.gol");
    let record = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();

    assert_eq!(record.schema_version, 1);
    assert_eq!(record.series, None);
    assert_eq!(record.result.status, "stable");
    assert_eq!(record.result.iterations_run, 0);
    assert_eq!(record.result.initial_alive_count, 4);
    assert_eq!(record.result.final_alive_count, 4);
}

#[test]
fn negative_unknown_magic_still_rejected() {
    let mut cursor = Cursor::new("GOL-RUN-RECORD v3\nschema_version: 3\n");
    let err = sniff_from_reader("dummy.gol", &mut cursor).unwrap_err();
    match err {
        MagicError::UnknownMagic { found, .. } => assert_eq!(found, "GOL-RUN-RECORD v3"),
        other => panic!("expected UnknownMagic; got {other:?}"),
    }
}

#[test]
fn series_array_shape_starts_at_generation_zero() {
    let series = fixture_series();
    let record = fixture_record(Some(series.clone()));

    assert_eq!(series.alive[0], record.result.initial_alive_count);
    assert_eq!(series.births[0], 0);
    assert_eq!(series.deaths[0], 0);
    assert_eq!(
        series.len(),
        usize::try_from(record.result.iterations_run).unwrap() + 1
    );
}

#[test]
fn sniff_v2_still_reports_run_record_kind() {
    let mut cursor = Cursor::new(format!("{RUN_RECORD_MAGIC_V2}\n"));
    assert_eq!(
        sniff_from_reader("dummy.gol", &mut cursor).unwrap(),
        FileKind::RunRecord
    );
}
