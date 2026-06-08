//! Game of Life library: core board, cell state, and generation logic.
//!
//! This library implements Conway's Game of Life with the following design:
//! - Bounded board (no toroidal wrapping)
//! - Single board copy with transitional cell states
//! - Two-pass generation: Mark (compute next state) + Normalize (finalize states)

use std::fmt;

/// Represents the state of a cell in the Game of Life.
///
/// - `Dead`: cell is dead
/// - `Alive`: cell is alive
/// - `Dying`: cell is alive but will become dead next generation (transitional state)
/// - `BecomingAlive`: cell is dead but will become alive next generation (transitional state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Dead,
    Alive,
    Dying,
    BecomingAlive,
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

                // Check bounds explicitly to avoid underflow issues
                if nx < 0 || nx >= self.width as i32 || ny < 0 || ny >= self.height as i32 {
                    // Out of bounds is dead
                    continue;
                }

                let state = self.get(nx as usize, ny as usize);
                // Treat Alive and Dying as originally live
                if state == CellState::Alive || state == CellState::Dying {
                    count += 1;
                }
            }
        }
        count
    }

    /// Advances the board by one generation using a two-pass algorithm:
    /// 1. Mark pass: compute each cell's next state using transitional states
    /// 2. Normalize pass: convert Dying -> Dead and BecomingAlive -> Alive
    ///
    /// After completion, the board contains only Dead and Alive states.
    pub fn advance_generation(&mut self) {
        // Mark pass: compute next state for each cell
        // Use a temporary vector to track state changes
        let mut next_states = vec![CellState::Dead; self.width * self.height];

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
                            CellState::BecomingAlive
                        } else {
                            CellState::Dead
                        }
                    }
                    // Transitional states should not occur at start of mark pass
                    CellState::Dying => {
                        if live_neighbors == 2 || live_neighbors == 3 {
                            CellState::Alive
                        } else {
                            CellState::Dying
                        }
                    }
                    CellState::BecomingAlive => {
                        if live_neighbors == 3 {
                            CellState::BecomingAlive
                        } else {
                            CellState::Dead
                        }
                    }
                };

                next_states[y * self.width + x] = next_state;
            }
        }

        // Apply mark pass results
        self.cells = next_states;

        // Normalize pass: convert transitional states to final states
        for y in 0..self.height {
            for x in 0..self.width {
                let state = self.get(x, y);
                let normalized = match state {
                    CellState::Dying => CellState::Dead,
                    CellState::BecomingAlive => CellState::Alive,
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
                    CellState::Alive => '█',
                    CellState::Dead => '·',
                    CellState::Dying => 'D',
                    CellState::BecomingAlive => 'B',
                };
                write!(f, "{}", ch)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_from_grid(lines: &[&str]) -> Board {
        let height = lines.len();
        let width = if height > 0 { lines[0].len() } else { 0 };
        let mut board = Board::new(width, height);

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let state = match ch {
                    '#' | '█' => CellState::Alive,
                    '.' | '·' | ' ' => CellState::Dead,
                    _ => CellState::Dead,
                };
                board.set(x, y, state);
            }
        }

        board
    }

    #[test]
    fn test_still_life_block() {
        // A 2x2 block should remain stable
        let mut board = board_from_grid(&["##", "##"]);

        let initial_state = board.clone();
        board.advance_generation();

        assert_eq!(board, initial_state, "Block should remain stable");
    }

    #[test]
    fn test_blinker_oscillator() {
        // Horizontal blinker in center of 3x3 board
        let mut board = board_from_grid(&["...", "###", "..."]);

        let initial = board.clone();

        // After one generation, should be vertical
        board.advance_generation();
        let expected_after_1 = board_from_grid(&[".#.", ".#.", ".#."]);
        assert_eq!(
            board, expected_after_1,
            "After 1 generation, blinker should be vertical"
        );

        // After two generations, should return to horizontal
        board.advance_generation();
        assert_eq!(
            board, initial,
            "After 2 generations, blinker should return to initial state"
        );
    }

    #[test]
    fn test_edge_cells_are_bounded() {
        // Test that out-of-bounds neighbors are treated as dead
        // A horizontal line of 3 should become a vertical line of 3
        let mut board = board_from_grid(&["...", "###", "..."]);

        board.advance_generation();

        let expected = board_from_grid(&[".#.", ".#.", ".#."]);
        assert_eq!(
            board, expected,
            "Edge cells should follow bounded board semantics"
        );
    }

    #[test]
    fn test_corner_cells_use_bounded_semantics() {
        // Single cell at corner should die (0 neighbors)
        let mut board = board_from_grid(&["#  ", "   ", "   "]);

        board.advance_generation();
        let expected = board_from_grid(&["   ", "   ", "   "]);
        assert_eq!(board, expected, "Single corner cell should die");
    }

    #[test]
    fn test_no_transitional_states_remain() {
        // After a complete generation, only Dead and Alive should exist
        let mut board = board_from_grid(&["###", "###", "###"]);

        board.advance_generation();

        for y in 0..board.height {
            for x in 0..board.width {
                let state = board.get(x, y);
                assert!(
                    state == CellState::Dead || state == CellState::Alive,
                    "Cell at ({}, {}) has transitional state: {:?}",
                    x,
                    y,
                    state
                );
            }
        }
    }

    #[test]
    fn test_neighbor_counting_during_mark_pass() {
        // Test that transitional states correctly preserve original neighbor count
        // during the mark pass.
        let mut board = board_from_grid(&["..#..", ".###.", ".....", ".....", "....."]);

        // This is the plus pattern. After advancing:
        // (2,0) has neighbors (2,1), (1,1), (3,1) = 3 -> Alive
        // (1,1) has neighbors (2,0), (2,1) = 2 -> Alive
        // (2,1) has neighbors (2,0), (1,1), (3,1) = 3 -> Alive
        // (3,1) has neighbors (2,0), (2,1) = 2 -> Alive
        // (1,0) has neighbors (2,0), (1,1), (2,1) = 3 -> BecomingAlive
        // (3,0) has neighbors (2,0), (2,1), (3,1) = 3 -> BecomingAlive
        // (2,2) has neighbors (1,1), (2,1), (3,1) = 3 -> BecomingAlive
        board.advance_generation();

        let expected = board_from_grid(&[".###.", ".###.", "..#..", ".....", "....."]);
        assert_eq!(
            board, expected,
            "Transitional states should preserve original neighbor count"
        );
    }
}
