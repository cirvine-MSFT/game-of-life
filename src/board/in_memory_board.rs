use std::convert::Infallible;
use std::fmt;
use std::mem;

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
        let cell_count = checked_cell_count(width, height)
            .expect("board dimensions exceed addressable cell capacity");
        Self {
            width,
            height,
            cells: vec![CellState::Dead; cell_count],
        }
    }

    /// Creates a new in-memory board if the allocation fits within `max_memory_bytes`.
    pub fn try_new(
        width: usize,
        height: usize,
        max_memory_bytes: usize,
    ) -> Result<Self, InMemoryBoardCreationError> {
        let requested_memory_bytes = checked_allocation_bytes(width, height)?;
        if requested_memory_bytes > max_memory_bytes {
            return Err(InMemoryBoardCreationError::MemoryBudgetExceeded {
                width,
                height,
                requested_memory_bytes,
                max_memory_bytes,
            });
        }

        Ok(Self::new(width, height))
    }

    /// Returns the bytes needed for the board's cell buffer.
    pub fn allocation_bytes(
        width: usize,
        height: usize,
    ) -> Result<usize, InMemoryBoardCreationError> {
        checked_allocation_bytes(width, height)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InMemoryBoardCreationError {
    CellCountOverflow {
        width: usize,
        height: usize,
    },
    AllocationSizeOverflow {
        width: usize,
        height: usize,
        cell_count: usize,
        cell_size: usize,
    },
    AllocationAddressSpaceExceeded {
        width: usize,
        height: usize,
        requested_memory_bytes: usize,
        max_addressable_bytes: usize,
    },
    MemoryBudgetExceeded {
        width: usize,
        height: usize,
        requested_memory_bytes: usize,
        max_memory_bytes: usize,
    },
}

impl fmt::Display for InMemoryBoardCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InMemoryBoardCreationError::CellCountOverflow { width, height } => write!(
                f,
                "Board size '{width}x{height}' is too large; width times height exceeds the supported cell capacity."
            ),
            InMemoryBoardCreationError::AllocationSizeOverflow {
                width,
                height,
                cell_count,
                cell_size,
            } => write!(
                f,
                "Board size '{width}x{height}' is too large; {cell_count} cells at {cell_size} bytes per cell exceed the supported allocation size."
            ),
            InMemoryBoardCreationError::AllocationAddressSpaceExceeded {
                width,
                height,
                requested_memory_bytes,
                max_addressable_bytes,
            } => write!(
                f,
                "Board size '{width}x{height}' requires {requested_memory_bytes} bytes, which exceeds the addressable allocation limit of {max_addressable_bytes} bytes."
            ),
            InMemoryBoardCreationError::MemoryBudgetExceeded {
                width,
                height,
                requested_memory_bytes,
                max_memory_bytes,
            } => write!(
                f,
                "Board size '{width}x{height}' requires {requested_memory_bytes} bytes, which exceeds the configured max board memory of {max_memory_bytes} bytes."
            ),
        }
    }
}

impl std::error::Error for InMemoryBoardCreationError {}

fn checked_cell_count(width: usize, height: usize) -> Result<usize, InMemoryBoardCreationError> {
    width
        .checked_mul(height)
        .ok_or(InMemoryBoardCreationError::CellCountOverflow { width, height })
}

fn checked_allocation_bytes(
    width: usize,
    height: usize,
) -> Result<usize, InMemoryBoardCreationError> {
    let cell_count = checked_cell_count(width, height)?;
    let cell_size = mem::size_of::<CellState>();
    let requested_memory_bytes = cell_count.checked_mul(cell_size).ok_or(
        InMemoryBoardCreationError::AllocationSizeOverflow {
            width,
            height,
            cell_count,
            cell_size,
        },
    )?;
    let max_addressable_bytes = isize::MAX as usize;
    if requested_memory_bytes > max_addressable_bytes {
        return Err(InMemoryBoardCreationError::AllocationAddressSpaceExceeded {
            width,
            height,
            requested_memory_bytes,
            max_addressable_bytes,
        });
    }

    Ok(requested_memory_bytes)
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
