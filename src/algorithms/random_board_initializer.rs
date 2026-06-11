use std::fmt;

use crate::board::{BoardEditor, CellCoordinate, CellState};

use super::BoardInitializer;

pub const DEFAULT_ALIVE_CELLS_PER_THOUSAND: u16 = 500;
pub const MAX_ALIVE_CELLS_PER_THOUSAND: u16 = 1000;

/// Seeded pseudo-random initializer for reproducible starting boards.
///
/// The initializer writes every in-bounds cell to either `Alive` or `Dead`.
/// Density is expressed as alive cells per 1,000 cells to avoid floating-point
/// configuration and keep simulations reproducible across platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomBoardInitializer {
    seed: u64,
    alive_cells_per_thousand: u16,
}

impl RandomBoardInitializer {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            alive_cells_per_thousand: DEFAULT_ALIVE_CELLS_PER_THOUSAND,
        }
    }

    pub fn with_alive_cells_per_thousand(
        seed: u64,
        alive_cells_per_thousand: u16,
    ) -> Result<Self, RandomBoardInitializerError> {
        if alive_cells_per_thousand > MAX_ALIVE_CELLS_PER_THOUSAND {
            return Err(RandomBoardInitializerError::AliveCellsPerThousandTooLarge {
                value: alive_cells_per_thousand,
            });
        }

        Ok(Self {
            seed,
            alive_cells_per_thousand,
        })
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn alive_cells_per_thousand(&self) -> u16 {
        self.alive_cells_per_thousand
    }
}

impl BoardInitializer for RandomBoardInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error> {
        let mut rng = SplitMix64::new(self.seed);

        for y in 0..board.height() {
            for x in 0..board.width() {
                let state = if rng.next_per_thousand() < self.alive_cells_per_thousand {
                    CellState::Alive
                } else {
                    CellState::Dead
                };
                board.set_cell(CellCoordinate::new(x, y), state)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomBoardInitializerError {
    AliveCellsPerThousandTooLarge { value: u16 },
}

impl fmt::Display for RandomBoardInitializerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RandomBoardInitializerError::AliveCellsPerThousandTooLarge { value } => write!(
                f,
                "Random board initializer density '{value}' is too large; use a value from 0 to {MAX_ALIVE_CELLS_PER_THOUSAND} alive cells per thousand."
            ),
        }
    }
}

impl std::error::Error for RandomBoardInitializerError {}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_per_thousand(&mut self) -> u16 {
        (self.next_u64() % MAX_ALIVE_CELLS_PER_THOUSAND as u64) as u16
    }
}
