use crate::board::{BoardEditor, CellCoordinate, CellState};

use super::BoardInitializer;

const DEMO_TILE_WIDTH: usize = 10;
const DEMO_TILE_HEIGHT: usize = 10;
const DEMO_TILE_GUTTER: usize = 2;
const DEMO_TILE_LIVE_CELLS: &[(usize, usize)] = &[
    (5, 2),
    (7, 2),
    (2, 3),
    (4, 3),
    (5, 3),
    (7, 3),
    (6, 4),
    (3, 5),
    (4, 5),
    (2, 6),
    (3, 6),
    (5, 6),
    (3, 7),
];

const SMALL_MOTIF_WIDTH: usize = 2;
const SMALL_MOTIF_HEIGHT: usize = 2;
const SMALL_MOTIF_GUTTER: usize = 2;
const SMALL_MOTIF_LIVE_CELLS: &[(usize, usize)] = &[(0, 0), (1, 0), (0, 1)];

/// Seeds a deterministic demo pattern that adapts to the board size.
///
/// Boards at least 10x10 receive one or more isolated 10x10 tiles that settle
/// within 20 generations. Smaller boards receive compact motifs that settle
/// quickly while still writing only in-bounds cells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DemoBoardInitializer;

impl BoardInitializer for DemoBoardInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error> {
        if board.width() >= DEMO_TILE_WIDTH && board.height() >= DEMO_TILE_HEIGHT {
            seed_repeated_pattern(
                board,
                DEMO_TILE_WIDTH,
                DEMO_TILE_HEIGHT,
                DEMO_TILE_GUTTER,
                DEMO_TILE_LIVE_CELLS,
            )
        } else {
            seed_repeated_pattern(
                board,
                SMALL_MOTIF_WIDTH,
                SMALL_MOTIF_HEIGHT,
                SMALL_MOTIF_GUTTER,
                SMALL_MOTIF_LIVE_CELLS,
            )
        }
    }
}

fn seed_repeated_pattern<B: BoardEditor + ?Sized>(
    board: &mut B,
    pattern_width: usize,
    pattern_height: usize,
    gutter: usize,
    live_cells: &[(usize, usize)],
) -> Result<(), B::Error> {
    if board.width() == 0 || board.height() == 0 {
        return Ok(());
    }

    if board.width() < pattern_width || board.height() < pattern_height {
        return seed_cropped_pattern(board, live_cells);
    }

    let tile_count_x = pattern_count(board.width(), pattern_width, gutter);
    let tile_count_y = pattern_count(board.height(), pattern_height, gutter);
    let span_x = pattern_span(tile_count_x, pattern_width, gutter);
    let span_y = pattern_span(tile_count_y, pattern_height, gutter);
    let offset_x = (board.width() - span_x) / 2;
    let offset_y = (board.height() - span_y) / 2;
    let stride_x = pattern_width + gutter;
    let stride_y = pattern_height + gutter;

    for tile_y in 0..tile_count_y {
        for tile_x in 0..tile_count_x {
            let tile_offset_x = offset_x + tile_x * stride_x;
            let tile_offset_y = offset_y + tile_y * stride_y;
            for &(x, y) in live_cells {
                board.set_cell(
                    CellCoordinate::new(tile_offset_x + x, tile_offset_y + y),
                    CellState::Alive,
                )?;
            }
        }
    }

    Ok(())
}

fn seed_cropped_pattern<B: BoardEditor + ?Sized>(
    board: &mut B,
    live_cells: &[(usize, usize)],
) -> Result<(), B::Error> {
    for &(x, y) in live_cells {
        if x < board.width() && y < board.height() {
            board.set_cell(CellCoordinate::new(x, y), CellState::Alive)?;
        }
    }
    Ok(())
}

fn pattern_count(board_length: usize, pattern_length: usize, gutter: usize) -> usize {
    (board_length + gutter) / (pattern_length + gutter)
}

fn pattern_span(count: usize, pattern_length: usize, gutter: usize) -> usize {
    count * pattern_length + (count - 1) * gutter
}
