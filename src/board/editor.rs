use super::{BoardView, CellCoordinate, CellState};

/// Mutable random-access board surface used by board initialization and update algorithms.
pub trait BoardEditor: BoardView {
    /// Sets the state of a cell. Implementations must ignore out-of-bounds coordinates.
    fn set_cell(&mut self, coordinate: CellCoordinate, state: CellState)
        -> Result<(), Self::Error>;
}
