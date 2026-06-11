use std::convert::Infallible;
use std::fmt;

use crate::algorithms::{BoardUpdater, InPlaceTransitionalUpdater};

use super::{BoardEditor, BoardView, CellCoordinate, CellState};

/// In-memory Game of Life board with finite boundaries.
///
/// The board uses a single `Vec<CellState>` buffer. The default updater uses
/// transitional states during generation advancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryBoard {
    width: usize,
    height: usize,
    cells: Vec<CellState>,
}

impl InMemoryBoard {
    /// Creates a new in-memory board with all cells dead.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![CellState::Dead; width * height],
        }
    }

    /// Returns the width of the board.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the board.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Gets the state of a cell. Returns `CellState::Dead` for out-of-bounds coordinates.
    pub fn get(&self, x: usize, y: usize) -> CellState {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x]
        } else {
            CellState::Dead
        }
    }

    /// Sets the state of a cell. Does nothing for out-of-bounds coordinates.
    pub fn set(&mut self, x: usize, y: usize, state: CellState) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = state;
        }
    }

    /// Advances the board by one generation using the default in-place updater.
    ///
    /// After completion, the board contains only Dead and Alive states.
    pub fn advance_generation(&mut self) {
        InPlaceTransitionalUpdater
            .advance_generation(self)
            .expect("in-memory board updates are infallible");
    }
}

impl BoardView for InMemoryBoard {
    type Error = Infallible;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn cell_state(&self, coordinate: CellCoordinate) -> Result<CellState, Self::Error> {
        Ok(self.get(coordinate.x, coordinate.y))
    }
}

impl BoardEditor for InMemoryBoard {
    fn set_cell(
        &mut self,
        coordinate: CellCoordinate,
        state: CellState,
    ) -> Result<(), Self::Error> {
        self.set(coordinate.x, coordinate.y, state);
        Ok(())
    }
}

impl fmt::Display for InMemoryBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                let ch = match self.get(x, y) {
                    CellState::Alive => '#',
                    CellState::Dead => '.',
                    CellState::Dying => 'D',
                    CellState::Resurrecting => 'B',
                };
                write!(f, "{ch}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
