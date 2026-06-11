/// Represents the state of a cell in the Game of Life.
///
/// - `Dead`: cell is dead
/// - `Alive`: cell is alive
/// - `Dying`: cell is alive but will become dead next generation (transitional state)
/// - `Resurrecting`: cell is dead but will become alive next generation (transitional state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Dead,
    Alive,
    Dying,
    Resurrecting,
}
