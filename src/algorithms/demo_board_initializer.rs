use crate::board::{BoardEditor, CellCoordinate, CellState};

use super::BoardInitializer;

const DEMO_PATTERN_WIDTH: usize = 10;
const DEMO_PATTERN_HEIGHT: usize = 10;
const DEMO_PATTERN_LIVE_CELLS: &[(usize, usize)] = &[
    (5, 0),
    (9, 1),
    (3, 2),
    (8, 2),
    (1, 3),
    (4, 3),
    (6, 4),
    (0, 5),
    (1, 5),
    (2, 5),
    (6, 5),
    (8, 5),
    (8, 6),
    (8, 7),
    (0, 8),
    (5, 8),
    (6, 8),
    (0, 9),
    (7, 9),
];

/// Seeds a deterministic 10x10 demo pattern that stabilizes within 20 generations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DemoBoardInitializer;

impl BoardInitializer for DemoBoardInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error> {
        let offset_x = board.width().saturating_sub(DEMO_PATTERN_WIDTH) / 2;
        let offset_y = board.height().saturating_sub(DEMO_PATTERN_HEIGHT) / 2;

        for &(x, y) in DEMO_PATTERN_LIVE_CELLS {
            let board_x = offset_x + x;
            let board_y = offset_y + y;
            if board_x < board.width() && board_y < board.height() {
                board.set_cell(CellCoordinate::new(board_x, board_y), CellState::Alive)?;
            }
        }

        Ok(())
    }
}
