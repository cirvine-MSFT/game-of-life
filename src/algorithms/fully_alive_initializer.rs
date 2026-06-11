use crate::board::{BoardEditor, CellCoordinate, CellState};

use super::BoardInitializer;

/// Seeds every in-bounds cell as alive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FullyAliveInitializer;

impl BoardInitializer for FullyAliveInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error> {
        for y in 0..board.height() {
            for x in 0..board.width() {
                board.set_cell(CellCoordinate::new(x, y), CellState::Alive)?;
            }
        }

        Ok(())
    }
}
