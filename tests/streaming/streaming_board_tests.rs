//! Integration tests for the file-streaming board backend.
//!
//! Coverage focuses on the bug categories surfaced by the design
//! review:
//!
//! - Stencil correctness at chunk boundaries (horizontal, vertical,
//!   corners).
//! - Small-board degenerate cases (1×1, 1×N, N×1, 2×2, 3×3).
//! - Owned-rectangle partitioning (every in-board cell updated
//!   exactly once per generation).
//! - Stats correctness across chunk boundaries.
//! - Row-band fast path vs general 2D path produce identical results.
//! - Edge-of-board cells use bounded semantics (out-of-bounds
//!   neighbors = Dead).

use std::env;
use std::fs;
use std::path::PathBuf;

use game_of_life::{
    derive_chunk_dimensions, BoardEditor, BoardView, CellCoordinate, CellState, InMemoryBoard,
    InPlaceTransitionalUpdater, StreamingBoard, StreamingBoardCreationError, StreamingBoardParams,
};

fn make_streaming(
    width: usize,
    height: usize,
    max_mem: usize,
    chunk_override: Option<(usize, usize)>,
    test_name: &str,
) -> StreamingBoard {
    let dir = env::temp_dir();
    let hint = format!("test-{test_name}");
    let params = StreamingBoardParams {
        width,
        height,
        max_board_memory_bytes: max_mem,
        working_dir: Some(&dir),
        scratch_name_hint: &hint,
        chunk_rows_override: chunk_override.map(|c| c.0),
        chunk_cols_override: chunk_override.map(|c| c.1),
    };
    StreamingBoard::new(params).expect("streaming board should be constructable")
}

fn in_memory_from_grid(lines: &[&str]) -> InMemoryBoard {
    let height = lines.len();
    let width = if height > 0 { lines[0].len() } else { 0 };
    let mut board = InMemoryBoard::new(width, height);
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            board.set(
                x,
                y,
                match ch {
                    '#' => CellState::Alive,
                    _ => CellState::Dead,
                },
            );
        }
    }
    board
}

fn seed_streaming_from_grid(board: &mut StreamingBoard, lines: &[&str]) {
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let state = match ch {
                '#' => CellState::Alive,
                _ => CellState::Dead,
            };
            board
                .set_cell(CellCoordinate::new(x, y), state)
                .expect("set_cell should succeed during seed");
        }
    }
    board.flush().expect("seed flush should succeed");
}

/// Snapshot a streaming board into an InMemoryBoard by reading every
/// cell. Uses cell_state when the cell is in the loaded chunk, and
/// slides via `read_cell_with_slide` otherwise.
fn snapshot_into_in_memory(board: &mut StreamingBoard) -> InMemoryBoard {
    let width = board.width();
    let height = board.height();
    let mut snapshot = InMemoryBoard::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let state = read_cell_with_slide(board, x, y);
            snapshot.set(x, y, state);
        }
    }
    snapshot
}

/// Read a cell from a streaming board non-destructively, sliding the
/// chunk if needed. Uses `StreamingBoard::peek_cell` so reads do not
/// disturb cell state.
fn read_cell_with_slide(board: &mut StreamingBoard, x: usize, y: usize) -> CellState {
    board
        .peek_cell(CellCoordinate::new(x, y))
        .expect("peek_cell should never fail on in-board coordinates")
}

/// Advance both a streaming board and an in-memory board the same way
/// and assert they end in the same state.
fn assert_streaming_matches_in_memory(
    pattern: &[&str],
    mem_budget: usize,
    chunk_override: Option<(usize, usize)>,
    test_name: &str,
    generations: usize,
) {
    let mut in_mem = in_memory_from_grid(pattern);
    let mut streaming = make_streaming(
        in_mem.width(),
        in_mem.height(),
        mem_budget,
        chunk_override,
        test_name,
    );
    seed_streaming_from_grid(&mut streaming, pattern);

    for _ in 0..generations {
        in_mem.advance_generation();
        streaming
            .advance_with_rule(&InPlaceTransitionalUpdater)
            .expect("streaming advance should succeed");
    }
    streaming
        .flush()
        .expect("flush after advance should succeed");

    let snapshot = snapshot_into_in_memory(&mut streaming);
    assert_eq!(
        snapshot, in_mem,
        "streaming final state must match in-memory for pattern {pattern:?} (mem_budget={mem_budget}, override={chunk_override:?})"
    );

    let scratch_path = streaming.scratch_path().to_path_buf();
    drop(streaming);
    fs::remove_file(scratch_path).ok();
}

#[test]
fn negative_streaming_creation_rejects_below_floor_budget() {
    let dir = env::temp_dir();
    let params = StreamingBoardParams {
        width: 10,
        height: 10,
        max_board_memory_bytes: 1,
        working_dir: Some(&dir),
        scratch_name_hint: "below-floor",
        chunk_rows_override: None,
        chunk_cols_override: None,
    };
    let err = StreamingBoard::new(params).expect_err("budget=1 byte must reject");
    assert!(matches!(
        err,
        StreamingBoardCreationError::InsufficientMemoryBudget { .. }
    ));
}

#[test]
fn streaming_chunk_dimensions_pick_row_band_when_budget_allows() {
    let (rows, cols) =
        derive_chunk_dimensions(4, 100, 1024, None, None).expect("derive should succeed");
    assert_eq!(cols, 4, "should use full width as row-band");
    assert!(rows >= 1);
}

#[test]
fn streaming_chunk_dimensions_fall_back_to_2d_when_budget_too_small_for_row_band() {
    let (rows, cols) =
        derive_chunk_dimensions(1000, 1000, 16, None, None).expect("minimum budget should succeed");
    assert_eq!(rows, 1);
    assert!(cols < 1000);
    assert!(cols >= 1);
}

#[test]
fn negative_streaming_chunk_dimensions_reject_budget_below_min() {
    let err =
        derive_chunk_dimensions(10, 10, 1, None, None).expect_err("below-min budget should reject");
    assert!(matches!(
        err,
        StreamingBoardCreationError::InsufficientMemoryBudget { .. }
    ));
}

#[test]
fn streaming_chunk_dimensions_honor_overrides() {
    let (rows, cols) =
        derive_chunk_dimensions(100, 100, usize::MAX, Some(5), Some(7)).expect("derive");
    assert_eq!((rows, cols), (5, 7));
}

#[test]
fn negative_streaming_creation_rejects_zero_dimensions() {
    let dir = env::temp_dir();
    let params = StreamingBoardParams {
        width: 0,
        height: 5,
        max_board_memory_bytes: 1024,
        working_dir: Some(&dir),
        scratch_name_hint: "zero-width",
        chunk_rows_override: None,
        chunk_cols_override: None,
    };
    let err = StreamingBoard::new(params).expect_err("width=0 must reject");
    assert!(matches!(
        err,
        StreamingBoardCreationError::InvalidDimensions { .. }
    ));
}

#[test]
fn edge_case_one_by_one_board_streams_correctly() {
    assert_streaming_matches_in_memory(&["#"], 1024, None, "1x1", 1);
}

#[test]
fn edge_case_one_by_n_board_streams_correctly() {
    assert_streaming_matches_in_memory(&["#.#.#"], 1024, None, "1xN", 2);
}

#[test]
fn edge_case_n_by_one_board_streams_correctly() {
    assert_streaming_matches_in_memory(&["#", ".", "#", ".", "#"], 1024, None, "Nx1", 2);
}

#[test]
fn edge_case_two_by_two_block_is_stable_streaming() {
    assert_streaming_matches_in_memory(&["##", "##"], 1024, None, "2x2-block", 3);
}

#[test]
fn edge_case_three_by_three_blinker_streaming() {
    assert_streaming_matches_in_memory(&["...", "###", "..."], 1024, None, "3x3-blinker", 2);
}

#[test]
fn row_band_fast_path_matches_in_memory_for_blinker() {
    assert_streaming_matches_in_memory(
        &[".....", "..#..", "..#..", "..#..", "....."],
        4096,
        None,
        "row-band-blinker",
        4,
    );
}

#[test]
fn general_2d_path_matches_in_memory_for_blinker() {
    // Force the general 2D path by overriding chunk_cols.
    assert_streaming_matches_in_memory(
        &[
            "..........",
            "..........",
            "...###....",
            "..........",
            "..........",
            "..........",
            "..........",
            "...###....",
            "..........",
            "..........",
        ],
        4096,
        Some((1, 4)),
        "2d-blinker",
        4,
    );
}

#[test]
fn blinker_straddling_horizontal_chunk_boundary_oscillates_correctly() {
    assert_streaming_matches_in_memory(
        &[".....", "..#..", "..#..", "..#..", "....."],
        4096,
        Some((1, 5)),
        "blinker-cross-row-band",
        4,
    );
}

#[test]
fn blinker_straddling_vertical_chunk_boundary_oscillates_correctly() {
    assert_streaming_matches_in_memory(
        &[".....", ".....", ".###.", ".....", "....."],
        4096,
        Some((3, 2)),
        "blinker-cross-vertical",
        4,
    );
}

#[test]
fn block_at_chunk_corner_remains_stable_streaming() {
    assert_streaming_matches_in_memory(
        &["......", "......", "..##..", "..##..", "......", "......"],
        4096,
        Some((2, 2)),
        "block-corner",
        3,
    );
}

#[test]
fn fully_alive_board_matches_in_memory_under_streaming() {
    assert_streaming_matches_in_memory(
        &["####", "####", "####", "####"],
        4096,
        Some((2, 2)),
        "fully-alive",
        1,
    );
}

#[test]
fn stats_match_in_memory_for_chunked_advance() {
    let pattern: &[&str] = &[
        "..........",
        "..#.......",
        "...#......",
        ".###......",
        "..........",
        "..........",
    ];
    let mut in_mem = in_memory_from_grid(pattern);
    let in_mem_outcome = in_mem.advance_generation();

    let mut streaming =
        make_streaming(in_mem.width(), in_mem.height(), 4096, Some((1, 4)), "stats");
    seed_streaming_from_grid(&mut streaming, pattern);
    let streaming_outcome = streaming
        .advance_with_rule(&InPlaceTransitionalUpdater)
        .expect("advance");

    assert_eq!(streaming_outcome.births, in_mem_outcome.births);
    assert_eq!(streaming_outcome.deaths, in_mem_outcome.deaths);
    assert_eq!(streaming_outcome.alive_count, in_mem_outcome.alive_count);

    let scratch_path = streaming.scratch_path().to_path_buf();
    drop(streaming);
    fs::remove_file(scratch_path).ok();
}

#[test]
fn working_dir_override_places_scratch_there() {
    let mut dir = env::temp_dir();
    dir.push(format!("gol-test-workdir-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create test workdir");

    let params = StreamingBoardParams {
        width: 4,
        height: 4,
        max_board_memory_bytes: 4096,
        working_dir: Some(&dir),
        scratch_name_hint: "workdir-test",
        chunk_rows_override: None,
        chunk_cols_override: None,
    };
    let board = StreamingBoard::new(params).expect("create");
    let scratch_path = board.scratch_path().to_path_buf();
    assert!(
        scratch_path.starts_with(&dir),
        "scratch path {scratch_path:?} should be under {dir:?}"
    );

    drop(board);
    fs::remove_file(&scratch_path).ok();
    fs::remove_dir(&dir).ok();
}

#[test]
fn working_dir_is_auto_created_if_missing() {
    // Users passing --working-dir /some/path don't expect to have to
    // mkdir it first; the streaming board should create it (matching
    // the OS-temp-dir default's "always exists" experience). The
    // original behavior surfaced an opaque os error 3 deep inside
    // ScratchFile::create.
    let mut base = env::temp_dir();
    base.push(format!(
        "gol-test-autocreate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    base.push("nested");
    base.push("does-not-exist");
    let _ = fs::remove_dir_all(&base);
    assert!(!base.exists(), "preconditions: dir should not exist yet");

    let params = StreamingBoardParams {
        width: 4,
        height: 4,
        max_board_memory_bytes: 4096,
        working_dir: Some(&base),
        scratch_name_hint: "autocreate",
        chunk_rows_override: None,
        chunk_cols_override: None,
    };
    let board = StreamingBoard::new(params).expect("create should auto-make the dir");
    assert!(base.exists(), "working dir should have been auto-created");
    let scratch_path = board.scratch_path().to_path_buf();
    drop(board);
    fs::remove_file(&scratch_path).ok();
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn concurrent_runs_in_same_workdir_get_unique_filenames() {
    let mut dir = env::temp_dir();
    dir.push(format!("gol-test-concurrent-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create dir");

    let mut paths: Vec<PathBuf> = Vec::new();
    let mut boards: Vec<StreamingBoard> = Vec::new();
    for i in 0..5 {
        let hint = format!("run-{i}");
        let params = StreamingBoardParams {
            width: 4,
            height: 4,
            max_board_memory_bytes: 4096,
            working_dir: Some(&dir),
            scratch_name_hint: &hint,
            chunk_rows_override: None,
            chunk_cols_override: None,
        };
        let board = StreamingBoard::new(params).expect("each board should create");
        paths.push(board.scratch_path().to_path_buf());
        boards.push(board);
    }

    paths.sort();
    for window in paths.windows(2) {
        assert_ne!(
            window[0], window[1],
            "concurrent scratch files must have distinct paths"
        );
    }

    drop(boards);
    for p in &paths {
        fs::remove_file(p).ok();
    }
    fs::remove_dir(&dir).ok();
}

#[test]
fn streaming_snapshot_writer_matches_in_memory_writer_byte_for_byte() {
    use game_of_life::persistence::{
        write_board_snapshot_to, write_streaming_board_snapshot_to, BoardSnapshot,
    };

    let pattern: &[&str] = &[
        "..........",
        "..#.......",
        "...#......",
        ".###......",
        "..........",
        "..........",
        ".....#....",
        "......#...",
        "....###...",
        "..........",
    ];
    let in_mem = in_memory_from_grid(pattern);
    let mut streaming = make_streaming(
        in_mem.width(),
        in_mem.height(),
        4096,
        Some((2, 4)),
        "snapshot-byte-eq",
    );
    seed_streaming_from_grid(&mut streaming, pattern);

    // Snapshot via the streaming writer.
    let mut streaming_bytes: Vec<u8> = Vec::new();
    write_streaming_board_snapshot_to(&mut streaming_bytes, &mut streaming)
        .expect("streaming writer should succeed");

    // Build the equivalent in-memory snapshot and write it. Both writers
    // use a freshly-grabbed `SystemTime::now()` for created_at, so we
    // can't compare those lines verbatim. We compare everything else.
    let snapshot = BoardSnapshot::for_board(in_mem);
    let mut in_mem_bytes: Vec<u8> = Vec::new();
    write_board_snapshot_to(&mut in_mem_bytes, &snapshot).expect("in-memory writer should succeed");

    // Strip the `created_at: ...` lines from each so the comparison is
    // timestamp-independent.
    let streaming_str = String::from_utf8(streaming_bytes).expect("utf8");
    let in_mem_str = String::from_utf8(in_mem_bytes).expect("utf8");

    let strip_created_at = |s: &str| -> String {
        s.lines()
            .filter(|line| !line.starts_with("created_at:"))
            .map(String::from)
            .collect::<Vec<_>>()
            .join("\n")
    };

    assert_eq!(
        strip_created_at(&streaming_str),
        strip_created_at(&in_mem_str),
        "streaming snapshot output should match in-memory snapshot output byte-for-byte (excluding timestamp)"
    );

    let scratch_path = streaming.scratch_path().to_path_buf();
    drop(streaming);
    fs::remove_file(scratch_path).ok();
}
