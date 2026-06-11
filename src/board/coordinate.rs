/// Identifies a cell by its zero-based board coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellCoordinate {
    pub x: usize,
    pub y: usize,
}

impl CellCoordinate {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}
