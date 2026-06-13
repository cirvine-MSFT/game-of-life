//! Unit tests for `game_of_life::persistence::board_snapshot` public API:
//! validate_loaded_board_size, suggest_memory_override, BoardSnapshot
//! read/write round-trips.

use std::sync::atomic::{AtomicU64, Ordering};

use game_of_life::persistence::{
    read_board_snapshot_default, suggest_memory_override, validate_loaded_board_size,
    write_board_snapshot, write_board_snapshot_to, BoardSnapshot, BoardSnapshotWriteError,
    LoadedBoardSizeError, BOARD_SNAPSHOT_MAGIC, SUGGESTED_MEMORY_OVERRIDE_FLOOR_BYTES,
};
use game_of_life::{CellCoordinate, CellState, InMemoryBoard};

// `BoardEditor::set_cell` lives in the board module; bring it in.
use game_of_life::BoardEditor;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!(
        "gol_board_snapshot_{label}_{}_{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn board_from_grid(lines: &[&str]) -> InMemoryBoard {
    let height = lines.len();
    let width = if height > 0 {
        lines[0].chars().count()
    } else {
        0
    };
    let mut board = InMemoryBoard::new(width, height);
    for (y, row) in lines.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let state = if ch == '#' {
                CellState::Alive
            } else {
                CellState::Dead
            };
            board.set_cell(CellCoordinate::new(x, y), state).ok();
        }
    }
    board
}

#[test]
fn validate_loaded_board_size_accepts_in_budget() {
    let required = validate_loaded_board_size(10, 10, 1024).unwrap();
    assert!(required <= 1024);
}

#[test]
fn negative_validate_loaded_board_size_rejects_over_budget() {
    // 1024x1024 cells * sizeof(CellState) >> 64 bytes.
    let err = validate_loaded_board_size(1024, 1024, 64).unwrap_err();
    match err {
        LoadedBoardSizeError::ExceedsMemoryBudget {
            width,
            height,
            max_budget_bytes,
            suggested_override,
            ..
        } => {
            assert_eq!(width, 1024);
            assert_eq!(height, 1024);
            assert_eq!(max_budget_bytes, 64);
            assert!(suggested_override.bytes >= 1024 * 1024);
        }
        other => panic!("expected ExceedsMemoryBudget, got {other:?}"),
    }
}

#[test]
fn negative_validate_loaded_board_size_rejects_unaddressable() {
    let huge = usize::MAX / 4 + 1;
    let err = validate_loaded_board_size(huge, huge, usize::MAX).unwrap_err();
    assert!(matches!(
        err,
        LoadedBoardSizeError::ExceedsAddressableMemory { .. }
    ));
}

#[test]
fn suggest_memory_override_returns_sensible_values() {
    let s = suggest_memory_override(0);
    assert!(s.bytes >= SUGGESTED_MEMORY_OVERRIDE_FLOOR_BYTES);
    let one_mb = suggest_memory_override(1024 * 1024);
    assert_eq!(one_mb.display.suffix, "MB");
    assert!(one_mb.bytes >= 1024 * 1024);
    let three_gb_request = suggest_memory_override(3 * 1024 * 1024 * 1024);
    assert_eq!(three_gb_request.display.suffix, "GB");
    assert!(three_gb_request.bytes >= 3 * 1024 * 1024 * 1024);
}

#[test]
fn write_snapshot_round_trip_small_board() {
    let board = board_from_grid(&[".#.", "###", "..."]);
    let snap = BoardSnapshot::for_board(board);
    let mut buf: Vec<u8> = Vec::new();
    write_board_snapshot_to(&mut buf, &snap).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.starts_with(BOARD_SNAPSHOT_MAGIC));
    assert!(text.contains("size: 3x3"));
    assert!(text.contains("alive_count: 4"));
    assert!(text.contains("dead_count: 5"));
    assert!(text.contains(".#."));
    assert!(text.contains("###"));
}

#[test]
fn write_then_read_snapshot_via_disk() {
    let board = board_from_grid(&["...", ".#.", "..."]);
    let snap = BoardSnapshot::for_board(board.clone());
    let dir = unique_temp_dir("round_trip");
    let path = dir.join("snap.gol");
    write_board_snapshot(&path, &snap).unwrap();
    let loaded = read_board_snapshot_default(&path, 64 * 1024).unwrap();
    assert_eq!(loaded.board, board);
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

#[test]
fn negative_write_refuses_to_overwrite_existing() {
    let dir = unique_temp_dir("overwrite");
    let path = dir.join("collision.gol");
    std::fs::write(&path, b"existing").unwrap();
    let snap = BoardSnapshot::for_board(board_from_grid(&["#"]));
    let err = write_board_snapshot(&path, &snap).unwrap_err();
    assert!(matches!(err, BoardSnapshotWriteError::OutputExists { .. }));
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}
