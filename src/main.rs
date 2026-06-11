//! Console application for Game of Life.
//!
//! Runs a built-in blinker pattern for a configured number of generations
//! and prints the final board state as ASCII.

use std::{env, process};

use game_of_life::{
    parse_cli_args, BoardInitializer, BoardUpdater, CenteredBlinkerInitializer, CliCommand,
    InMemoryBoard, InPlaceTransitionalUpdater, SimulationConfig,
};

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
    let mut board = InMemoryBoard::new(config.board_size.width, config.board_size.height);
    CenteredBlinkerInitializer
        .initialize(&mut board)
        .expect("in-memory board initialization is infallible");
    let updater = InPlaceTransitionalUpdater;

    for _ in 0..config.max_iterations {
        updater
            .advance_generation(&mut board)
            .expect("in-memory board updates are infallible");
    }

    println!("Game of Life");
    println!("Board size: {}", config.board_size);
    println!("Max iterations: {}", config.max_iterations);
    println!("Generation 0: fixed initial state seeded");
    println!("Final board state:");
    print!("{board}");
    println!("Simulation complete: {} iterations", config.max_iterations);
}

fn print_help() {
    print!("{HELP_TEXT}");
}
