use super::{BoardView, CellCoordinate, CellState};

/// Mutable random-access board surface used by board initialization and update algorithms.
pub trait BoardEditor: BoardView {
    /// Sets the state of a cell. Implementations must ignore out-of-bounds coordinates.
    fn set_cell(&mut self, coordinate: CellCoordinate, state: CellState)
        -> Result<(), Self::Error>;

    /// Sets every in-bounds cell to the same state.
    ///
    /// The default implementation writes through `set_cell`; concrete boards can
    /// override this for storage-specific bulk initialization.
    fn fill_cells(&mut self, state: CellState) -> Result<(), Self::Error> {
        for y in 0..self.height() {
            for x in 0..self.width() {
                self.set_cell(CellCoordinate::new(x, y), state)?;
            }
        }

        Ok(())
    }
}
