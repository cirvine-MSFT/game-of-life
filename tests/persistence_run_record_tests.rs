//! Unit tests for `game_of_life::persistence::run_record` public API:
//! write, read, integrity, extract — all driven by struct fixtures rather
//! than by the CLI (CLI-driven flows live in tests/persistence_cli_tests.rs).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use game_of_life::persistence::{
    board_grid_hash, extract_board_from_run, read_run_record, read_run_record_with_warnings,
    write_run_record, ContentHashMode, ExtractWhich, RunId, RunRecord, RunRecordConfig,
    RunRecordReadError, RunRecordResult, RunRecordWriteError, BOARD_SNAPSHOT_MAGIC,
    DEFAULT_MAX_INPUT_FILE_BYTES, SCHEMA_VERSION, TOOL_VERSION,
};
use game_of_life::{CellState, InMemoryBoard};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(label: &str) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!(
        "gol_run_record_{label}_{}_{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn small_board(width: usize, height: usize, alive_at: &[(usize, usize)]) -> InMemoryBoard {
    let mut board = InMemoryBoard::new(width, height);
    for (x, y) in alive_at {
        board.set(*x, *y, CellState::Alive);
    }
    board
}

fn fixture_record() -> RunRecord {
    let run_id = RunId::from_bytes([
        0x7b, 0x3a, 0x1f, 0x0c, 0x4d, 0x2e, 0x4a, 0x51, 0x9c, 0x5e, 0x2f, 0x8c, 0x3a, 0x1b, 0x9d,
        0x77,
    ]);
    let initial = small_board(3, 3, &[(0, 0), (1, 1), (2, 2)]);
    let final_ = small_board(3, 3, &[(1, 1)]);
    let initial_hash = board_grid_hash(&initial);
    let final_hash = board_grid_hash(&final_);
    RunRecord {
        run_id,
        schema_version: SCHEMA_VERSION,
        created_at: UNIX_EPOCH + Duration::from_secs(1_780_000_000),
        tool_version: TOOL_VERSION.to_string(),
        config: RunRecordConfig {
            board_size: (3, 3),
            max_iterations: 10,
            max_board_memory_bytes: 64 * 1024 * 1024,
            initial_board_source: "random".to_string(),
            random_seed: 42,
            updater: "in_place_transitional".to_string(),
            continued_from: None,
        },
        result: RunRecordResult {
            status: "max_iterations".to_string(),
            iterations_run: 10,
            wall_time_ms: 3,
            initial_alive_count: 3,
            final_alive_count: 1,
            peak_alive_count: 3,
            peak_alive_generation: 0,
            min_alive_count: 1,
            min_alive_generation: 5,
            total_births: 4,
            total_deaths: 6,
            initial_board_hash: initial_hash,
            final_board_hash: final_hash,
        },
        initial_board: initial,
        final_board: final_,
    }
}

fn write_to_temp(label: &str, record: &RunRecord) -> PathBuf {
    let dir = unique_temp_dir(label);
    let path = dir.join(format!("{label}.gol"));
    write_run_record(&path, record).unwrap();
    path
}

#[test]
fn round_trip_writes_then_reads_identical_record() {
    let original = fixture_record();
    let path = write_to_temp("round_trip", &original);
    let loaded = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();
    assert_eq!(loaded.run_id, original.run_id);
    assert_eq!(loaded.config.board_size, original.config.board_size);
    assert_eq!(loaded.config.random_seed, original.config.random_seed);
    assert_eq!(loaded.config.continued_from, original.config.continued_from);
    assert_eq!(loaded.result.status, original.result.status);
    assert_eq!(loaded.result.iterations_run, original.result.iterations_run);
    assert_eq!(loaded.initial_board, original.initial_board);
    assert_eq!(loaded.final_board, original.final_board);
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(path.parent().unwrap()).ok();
}

#[test]
fn round_trip_writes_then_reads_with_continued_from() {
    let mut original = fixture_record();
    original.config.continued_from = Some(RunId::from_bytes([0xaa; 16]));
    let path = write_to_temp("continued", &original);
    let loaded = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();
    assert_eq!(loaded.config.continued_from, original.config.continued_from);
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(path.parent().unwrap()).ok();
}

#[test]
fn negative_read_detects_corruption() {
    let original = fixture_record();
    let path = write_to_temp("corruption", &original);
    let body = std::fs::read_to_string(&path).unwrap();
    let corrupted = body.replacen("...\n.#.\n...", "...\n##.\n...", 1);
    assert_ne!(
        corrupted, body,
        "test fixture must produce a non-trivial mutation"
    );
    std::fs::write(&path, corrupted).unwrap();
    let err = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap_err();
    assert!(matches!(err, RunRecordReadError::Corrupted { .. }));
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(path.parent().unwrap()).ok();
}

#[test]
fn ignore_integrity_downgrades_corruption_to_warning() {
    let original = fixture_record();
    let path = write_to_temp("ignore_integrity", &original);
    let body = std::fs::read_to_string(&path).unwrap();
    let corrupted = body.replacen("...\n.#.\n...", "...\n##.\n...", 1);
    // alive_count was 1 -> 2, dead_count was 8 -> 7; fix headers so we
    // exercise the integrity check, not the header validator.
    let corrupted = corrupted.replacen(
        "alive_count: 1\ndead_count: 8",
        "alive_count: 2\ndead_count: 7",
        1,
    );
    std::fs::write(&path, corrupted).unwrap();
    let loaded = read_run_record_with_warnings(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Ignore,
    )
    .unwrap();
    assert!(!loaded.warnings.is_empty());
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(path.parent().unwrap()).ok();
}

#[test]
fn negative_read_missing_content_hash_under_enforce() {
    let original = fixture_record();
    let path = write_to_temp("missing_trailer", &original);
    let body = std::fs::read_to_string(&path).unwrap();
    let truncated = body
        .lines()
        .filter(|l| !l.starts_with("content_hash:"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, format!("{truncated}\n")).unwrap();
    let err = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap_err();
    assert!(matches!(err, RunRecordReadError::MissingContentHash { .. }));
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(path.parent().unwrap()).ok();
}

#[test]
fn crlf_rewrite_does_not_break_integrity_check() {
    let original = fixture_record();
    let path = write_to_temp("crlf", &original);
    let body = std::fs::read_to_string(&path).unwrap();
    let crlf = body.replace('\n', "\r\n");
    std::fs::write(&path, crlf).unwrap();
    let loaded = read_run_record(
        &path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();
    assert_eq!(loaded.initial_board, original.initial_board);
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(path.parent().unwrap()).ok();
}

#[test]
fn extract_board_round_trip_via_disk() {
    let original = fixture_record();
    let run_path = write_to_temp("extract_source", &original);
    let out_dir = run_path.parent().unwrap().to_path_buf();
    let out_path = out_dir.join("extracted.gol");
    extract_board_from_run(
        &run_path,
        ExtractWhich::Final,
        &out_path,
        64 * 1024 * 1024,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        ContentHashMode::Enforce,
    )
    .unwrap();
    let body = std::fs::read_to_string(&out_path).unwrap();
    assert!(body.starts_with(BOARD_SNAPSHOT_MAGIC));
    assert!(!body.contains("content_hash:"));
    std::fs::remove_file(&run_path).ok();
    std::fs::remove_file(&out_path).ok();
    std::fs::remove_dir(out_dir).ok();
}

#[test]
fn negative_write_refuses_to_overwrite_existing() {
    let dir = unique_temp_dir("overwrite");
    let path = dir.join("collision.gol");
    std::fs::write(&path, b"existing").unwrap();
    let original = fixture_record();
    let err = write_run_record(&path, &original).unwrap_err();
    assert!(matches!(err, RunRecordWriteError::OutputExists { .. }));
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}
