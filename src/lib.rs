//! Game of Life library: core board, configuration, and generation logic.
//!
//! This library implements Conway's Game of Life with the following design:
//! - Bounded board (no toroidal wrapping)
//! - Trait-based board initialization and update algorithms
//! - Default single board copy update with transitional cell states
//! - Two-pass default generation: Mark (compute next state) + Normalize (finalize states)

pub mod algorithms;
pub mod board;
pub mod config;

pub use algorithms::{
    BoardInitializer, BoardUpdater, CenteredBlinkerInitializer, InPlaceTransitionalUpdater,
    RandomBoardInitializer, RandomBoardInitializerError, DEFAULT_ALIVE_CELLS_PER_THOUSAND,
    MAX_ALIVE_CELLS_PER_THOUSAND,
};
pub use board::{BoardEditor, BoardView, CellCoordinate, CellState, InMemoryBoard};
pub use config::{
    parse_cli_args, BoardSize, BoardSizeParseError, CliCommand, ConfigError, IterationParseError,
    SimulationConfig,
};
