//! Board model, board interfaces, and concrete board implementations.

mod cell_state;
mod coordinate;
mod editor;
mod in_memory_board;
mod view;

pub use cell_state::CellState;
pub use coordinate::CellCoordinate;
pub use editor::BoardEditor;
pub use in_memory_board::InMemoryBoard;
pub use view::BoardView;
