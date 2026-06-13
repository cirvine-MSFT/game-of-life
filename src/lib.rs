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
pub mod persistence;
pub mod stats;

pub use algorithms::{
    BlinkerBoardInitializer, BoardInitializer, BoardUpdater, CenteredBlinkerInitializer,
    DemoBoardInitializer, FullyAliveInitializer, InPlaceTransitionalUpdater,
    RandomBoardInitializer, RandomBoardInitializerError, DEFAULT_ALIVE_CELLS_PER_THOUSAND,
    MAX_ALIVE_CELLS_PER_THOUSAND,
};
pub use board::{
    BoardEditor, BoardView, CellCoordinate, CellState, InMemoryBoard, InMemoryBoardCreationError,
};
pub use config::{
    parse_cli_args, parse_memory_size, BoardSize, BoardSizeParseError, CliCommand, ConfigError,
    ContinuationBudget, ExtractBoardConfig, InitialBoardSource, InitialBoardSourceParseError,
    InitialBoardSpec, IntegrityMode, IterationParseError, LoadFrom, LoadFromParseError,
    MemorySizeParseError, ReplayConfig, SaveSettings, SimulationConfig,
    DEFAULT_MAX_BOARD_MEMORY_BYTES, DEFAULT_RUNS_DIR,
};
pub use stats::{AdvanceOutcome, RunStatistics, RunStatisticsCollector};
