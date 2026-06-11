use super::{CellCoordinate, CellState};

/// Read-only random-access board surface used by board algorithms.
pub trait BoardView {
    type Error;

    fn width(&self) -> usize;

    fn height(&self) -> usize;

    /// Gets the state of a cell. Implementations must return `CellState::Dead`
    /// for out-of-bounds coordinates.
    fn cell_state(&self, coordinate: CellCoordinate) -> Result<CellState, Self::Error>;

    fn read_cells(
        &self,
        coordinates: &[CellCoordinate],
        states: &mut Vec<CellState>,
    ) -> Result<(), Self::Error> {
        states.clear();
        for coordinate in coordinates {
            states.push(self.cell_state(*coordinate)?);
        }
        Ok(())
    }
}
