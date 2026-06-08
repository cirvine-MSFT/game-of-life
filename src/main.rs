//! Console application for Game of Life.
//!
//! Runs a built-in pattern (5x5 blinker) for a fixed number of generations
//! and prints each board state as ASCII.

use game_of_life::{Board, CellState};

fn main() {
    const WIDTH: usize = 5;
    const HEIGHT: usize = 5;
    const GENERATIONS: usize = 10;

    // Initialize board with a simple pattern (blinker in the center)
    let mut board = Board::new(WIDTH, HEIGHT);

    // Create a 3-cell horizontal blinker at the center
    board.set(1, 2, CellState::Alive);
    board.set(2, 2, CellState::Alive);
    board.set(3, 2, CellState::Alive);

    println!("Game of Life - 5x5 Blinker Pattern");
    println!("====================================\n");

    // Print initial state
    println!("Generation 0:");
    println!("{}", board);
    println!();

    // Advance and print each generation
    for gen in 1..=GENERATIONS {
        board.advance_generation();
        println!("Generation {}:", gen);
        println!("{}", board);
        println!();
    }

    println!("====================================");
    println!("Simulation complete: {} generations", GENERATIONS);
}
