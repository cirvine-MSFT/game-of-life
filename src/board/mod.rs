//! Board model, board interfaces, and concrete board implementations.

mod board_size;
mod cell_state;
mod coordinate;
mod editor;
mod in_memory_board;
mod streaming_board;
mod view;

pub use board_size::{BoardSize, BoardSizeParseError, DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_WIDTH};
pub use cell_state::CellState;
pub use coordinate::CellCoordinate;
pub use editor::{default_advance_with_rule, BoardEditor};
pub use in_memory_board::{InMemoryBoard, InMemoryBoardCreationError};
pub use streaming_board::{
    derive_chunk_dimensions, StreamingBoard, StreamingBoardCreationError, StreamingBoardParams,
};
pub use view::BoardView;
