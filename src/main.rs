//! Console application for Game of Life.
//!
//! Runs a built-in blinker pattern for a configured number of generations
//! and prints the final board state as ASCII.

use std::{env, process};

use game_of_life::{parse_cli_args, Board, CellState, CliCommand, SimulationConfig};

const HELP_TEXT: &str = concat!(
    "Game of Life\n",
    "\n",
    "Usage:\n",
    "  game-of-life [OPTIONS]\n",
    "\n",
    "Options:\n",
    "  -h, --help                         Print this help message.\n",
    "  -b, --board-size <WIDTHxHEIGHT>    Set the 2D board size, for example 5x5.\n",
    "  -m, --max-iterations <COUNT>       Set generations to run; 0 prints the initial board.\n",
    "\n",
    "Defaults:\n",
    "  --board-size 5x5\n",
    "  --max-iterations 10\n",
);

fn main() {
    match parse_cli_args(env::args().skip(1)) {
        Ok(CliCommand::Help) => {
            print_help();
        }
        Ok(CliCommand::Run(config)) => {
            run_simulation(config);
        }
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Use --help to see usage and supported options.");
            process::exit(2);
        }
    }
}

fn run_simulation(config: SimulationConfig) {
    let mut board = Board::new(config.board_size.width, config.board_size.height);
    seed_fixed_blinker(&mut board);

    for _ in 0..config.max_iterations {
        board.advance_generation();
    }

    println!("Game of Life");
    println!("Board size: {}", config.board_size);
    println!("Max iterations: {}", config.max_iterations);
    println!("Final board state:");
    print!("{board}");
    println!("Simulation complete: {} iterations", config.max_iterations);
}

fn seed_fixed_blinker(board: &mut Board) {
    let center_x = board.width() / 2;
    let center_y = board.height() / 2;

    for dx in [-1, 0, 1] {
        let x = center_x as isize + dx;
        if x >= 0 {
            board.set(x as usize, center_y, CellState::Alive);
        }
    }
}

fn print_help() {
    print!("{HELP_TEXT}");
}
