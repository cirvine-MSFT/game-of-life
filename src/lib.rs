//! Game of Life library: core board, configuration, and generation logic.
//!
//! This library implements Conway's Game of Life with the following design:
//! - Bounded board (no toroidal wrapping)
//! - Single board copy with transitional cell states
//! - Two-pass generation: Mark (compute next state) + Normalize (finalize states)

pub mod board;
pub mod config;

pub use board::{Board, CellState};
pub use config::{
    parse_cli_args, BoardSize, BoardSizeParseError, CliCommand, ConfigError, IterationParseError,
    SimulationConfig,
};
