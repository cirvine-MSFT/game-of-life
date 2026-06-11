//! Console application for Game of Life.
//!
//! Runs a configured initial board for a configured number of generations
//! and prints the final board state as ASCII.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, process};

use game_of_life::{
    parse_cli_args, BlinkerBoardInitializer, BoardInitializer, BoardUpdater, CliCommand,
    DemoBoardInitializer, FullyAliveInitializer, InMemoryBoard, InMemoryBoardCreationError,
    InPlaceTransitionalUpdater, InitialBoardSource, RandomBoardInitializer, SimulationConfig,
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
    "      --max-board-memory <SIZE>      Set max in-memory board budget, for example 64MB.\n",
    "      --initial-board <SOURCE>       Set initial board source: demo, alive, blinker, or random.\n",
    "\n",
    "Defaults:\n",
    "  --board-size 10x10\n",
    "  --max-iterations 10\n",
    "  --max-board-memory 64MB\n",
    "  --initial-board demo\n",
);

fn main() {
    match parse_cli_args(env::args().skip(1)) {
        Ok(CliCommand::Help) => {
            print_help();
        }
        Ok(CliCommand::Run(config)) => {
            if let Err(error) = run_simulation(config) {
                eprintln!("Error: {error}");
                eprintln!("Use --help to see usage and supported options.");
                process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Use --help to see usage and supported options.");
            process::exit(2);
        }
    }
}

fn run_simulation(config: SimulationConfig) -> Result<(), InMemoryBoardCreationError> {
    let mut board = InMemoryBoard::try_new(
        config.board_size.width,
        config.board_size.height,
        config.max_board_memory_bytes,
    )?;
    initialize_board(config.initial_board, &mut board);
    let updater = InPlaceTransitionalUpdater;

    for _ in 0..config.max_iterations {
        updater
            .advance_generation(&mut board)
            .expect("in-memory board updates are infallible");
    }

    println!("Game of Life");
    println!("Board size: {}", config.board_size);
    println!("Max iterations: {}", config.max_iterations);
    println!("Max board memory: {} bytes", config.max_board_memory_bytes);
    println!("Initial board: {}", config.initial_board);
    println!(
        "Generation 0: '{}' initial board seeded",
        config.initial_board
    );
    println!("Final board state:");
    print!("{board}");
    println!("Simulation complete: {} iterations", config.max_iterations);
    Ok(())
}

fn initialize_board(source: InitialBoardSource, board: &mut InMemoryBoard) {
    match source {
        InitialBoardSource::Demo => DemoBoardInitializer
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
        InitialBoardSource::Alive => FullyAliveInitializer
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
        InitialBoardSource::Blinker => BlinkerBoardInitializer
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
        InitialBoardSource::Random => RandomBoardInitializer::new(generate_random_seed())
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
    }
}

fn generate_random_seed() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    hasher.write_u128(now);
    hasher.write_u32(process::id());
    hasher.finish()
}

fn print_help() {
    print!("{HELP_TEXT}");
}
