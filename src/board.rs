use std::fmt;

/// Represents the state of a cell in the Game of Life.
///
/// - `Dead`: cell is dead
/// - `Alive`: cell is alive
/// - `Dying`: cell is alive but will become dead next generation (transitional state)
/// - `Resurrecting`: cell is dead but will become alive next generation (transitional state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Dead,
    Alive,
    Dying,
    Resurrecting,
}

/// Represents the Game of Life board with finite boundaries.
///
/// The board uses a single buffer with transitional states during generation advancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    width: usize,
    height: usize,
    cells: Vec<CellState>,
}

impl Board {
    /// Creates a new board with all cells dead.
    pub fn new(width: usize, height: usize) -> Self {
        Board {
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

    /// Counts the number of live neighbors (Alive and Dying states).
    /// Out-of-bounds neighbors are considered dead.
    fn count_live_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for dy in [-1, 0, 1] {
            for dx in [-1, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx < 0 || nx >= self.width as i32 || ny < 0 || ny >= self.height as i32 {
                    continue;
                }

                let state = self.get(nx as usize, ny as usize);
                if state == CellState::Alive || state == CellState::Dying {
                    count += 1;
                }
            }
        }
        count
    }

    /// Advances the board by one generation using a two-pass algorithm:
    /// 1. Mark pass: compute each cell's next state using transitional states, in-place
    ///    During this pass, Alive|Dying count as "originally live", Dead|Resurrecting as "originally dead"
    /// 2. Normalize pass: convert Dying -> Dead and Resurrecting -> Alive
    ///
    /// After completion, the board contains only Dead and Alive states.
    /// No second board copy is allocated; all changes are made in-place via transitional states.
    pub fn advance_generation(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let current = self.get(x, y);
                let live_neighbors = self.count_live_neighbors(x, y);

                let next_state = match current {
                    CellState::Alive => {
                        if live_neighbors == 2 || live_neighbors == 3 {
                            CellState::Alive
                        } else {
                            CellState::Dying
                        }
                    }
                    CellState::Dead => {
                        if live_neighbors == 3 {
                            CellState::Resurrecting
                        } else {
                            CellState::Dead
                        }
                    }
                    CellState::Dying => {
                        if live_neighbors == 2 || live_neighbors == 3 {
                            CellState::Alive
                        } else {
                            CellState::Dying
                        }
                    }
                    CellState::Resurrecting => {
                        if live_neighbors == 3 {
                            CellState::Resurrecting
                        } else {
                            CellState::Dead
                        }
                    }
                };

                self.set(x, y, next_state);
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let state = self.get(x, y);
                let normalized = match state {
                    CellState::Dying => CellState::Dead,
                    CellState::Resurrecting => CellState::Alive,
                    other => other,
                };
                self.set(x, y, normalized);
            }
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                let ch = match self.get(x, y) {
                    CellState::Alive => '#',
                    CellState::Dead => '.',
                    CellState::Dying => 'D',
                    CellState::Resurrecting => 'B',
                };
                write!(f, "{}", ch)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
