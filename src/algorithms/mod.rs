//! Board initialization and update algorithms.

mod board_initializer;
mod board_updater;
mod centered_blinker_initializer;
mod demo_board_initializer;
mod in_place_transitional_updater;
mod random_board_initializer;

pub use board_initializer::BoardInitializer;
pub use board_updater::BoardUpdater;
pub use centered_blinker_initializer::CenteredBlinkerInitializer;
pub use demo_board_initializer::DemoBoardInitializer;
pub use in_place_transitional_updater::InPlaceTransitionalUpdater;
pub use random_board_initializer::{
    RandomBoardInitializer, RandomBoardInitializerError, DEFAULT_ALIVE_CELLS_PER_THOUSAND,
    DEFAULT_RANDOM_SEED, MAX_ALIVE_CELLS_PER_THOUSAND,
};
