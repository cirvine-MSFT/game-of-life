use crate::board::{BoardEditor, CellCoordinate, CellState};

use super::BoardInitializer;

/// Seeds a horizontal three-cell blinker centered on the board when possible.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CenteredBlinkerInitializer;

impl BoardInitializer for CenteredBlinkerInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error> {
        let width = board.width();
        let height = board.height();
        if width == 0 || height == 0 {
            return Ok(());
        }

        let center_x = width / 2;
        let center_y = height / 2;

        for dx in [-1isize, 0, 1] {
            if let Some(x) = center_x.checked_add_signed(dx) {
                if x < width {
                    board.set_cell(CellCoordinate::new(x, center_y), CellState::Alive)?;
                }
            }
        }
        Ok(())
    }
}
