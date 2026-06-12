use crate::board::{BoardEditor, CellState};

use super::BoardInitializer;

/// Seeds every in-bounds cell as alive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FullyAliveInitializer;

impl BoardInitializer for FullyAliveInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error> {
        board.fill_cells(CellState::Alive)
    }
}
