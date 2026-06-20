use crate::stats::AdvanceOutcome;

use super::{BoardView, CellCoordinate, CellState};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoardSignature {
    width: usize,
    height: usize,
    alive_count: u64,
    cells: Vec<u8>,
}

impl BoardSignature {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn alive_count(&self) -> u64 {
        self.alive_count
    }

    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    pub fn from_view<B: BoardView + ?Sized>(board: &B) -> Result<Self, B::Error> {
        let mut accumulator = BoardSignatureAccumulator::new(board.width(), board.height());
        for y in 0..board.height() {
            for x in 0..board.width() {
                let coordinate = CellCoordinate::new(x, y);
                accumulator.observe(coordinate, board.cell_state(coordinate)?);
            }
        }
        Ok(accumulator.finish())
    }
}

#[derive(Debug, Clone)]
pub struct BoardSignatureAccumulator {
    width: usize,
    height: usize,
    alive_count: u64,
    cells: Vec<u8>,
}

impl BoardSignatureAccumulator {
    pub fn new(width: usize, height: usize) -> Self {
        let cell_count = width.saturating_mul(height);
        let byte_count = cell_count.saturating_add(7) / 8;
        Self {
            width,
            height,
            alive_count: 0,
            cells: vec![0; byte_count],
        }
    }

    pub fn observe(&mut self, coordinate: CellCoordinate, state: CellState) {
        if coordinate.x >= self.width || coordinate.y >= self.height {
            return;
        }
        if matches!(state.normalized(), CellState::Alive) {
            self.alive_count += 1;
            let index = coordinate.y * self.width + coordinate.x;
            self.cells[index / 8] |= 1 << (index % 8);
        }
    }

    pub fn finish(self) -> BoardSignature {
        BoardSignature {
            width: self.width,
            height: self.height,
            alive_count: self.alive_count,
            cells: self.cells,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationSummary {
    pub outcome: AdvanceOutcome,
    pub signature: Option<BoardSignature>,
}

impl GenerationSummary {
    pub fn new(outcome: AdvanceOutcome, signature: Option<BoardSignature>) -> Self {
        Self { outcome, signature }
    }
}

pub trait BoardSignatureSource {
    type Error;

    fn board_signature(&mut self) -> Result<BoardSignature, Self::Error>;
}
