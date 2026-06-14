/// Represents the state of a cell in the Game of Life.
///
/// - `Dead`: cell is dead
/// - `Alive`: cell is alive
/// - `Dying`: cell is alive but will become dead next generation (transitional state)
/// - `Resurrecting`: cell is dead but will become alive next generation (transitional state)
///
/// `Dying` and `Resurrecting` are *transitional* states used internally by board
/// backends during the two-phase transitional-state generation algorithm. Outside
/// a generation, every cell is either `Dead` or `Alive`; rules (the [`CellRule`]
/// trait) never see transitional values.
///
/// [`CellRule`]: crate::algorithms::CellRule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Dead,
    Alive,
    Dying,
    Resurrecting,
}

impl CellState {
    /// True if this cell value represents a cell that was alive at the start
    /// of the current generation.
    ///
    /// During a mark pass, neighbor cells may have already been overwritten
    /// with their transitional next state. This helper recovers the
    /// originally-alive truth so neighbor counting is consistent regardless
    /// of iteration order. It is the single source of truth for every board
    /// backend's mark pass.
    ///
    /// | variant        | originally alive? |
    /// |----------------|-------------------|
    /// | `Alive`        | yes (still alive) |
    /// | `Dying`        | yes (was alive, will be dead) |
    /// | `Dead`         | no                |
    /// | `Resurrecting` | no (was dead, will be alive)  |
    pub fn is_originally_alive(self) -> bool {
        matches!(self, CellState::Alive | CellState::Dying)
    }

    /// Convert a possibly-transitional cell state to its final two-state form.
    /// `Dying` becomes `Dead`; `Resurrecting` becomes `Alive`; `Dead` and
    /// `Alive` are unchanged.
    pub fn normalized(self) -> CellState {
        match self {
            CellState::Dying => CellState::Dead,
            CellState::Resurrecting => CellState::Alive,
            other => other,
        }
    }

    /// Build the transitional state that encodes a per-cell mark-phase
    /// transition. The result tracks the originally-alive bit (via
    /// [`CellState::is_originally_alive`]) and the rule's next-alive output,
    /// so a later normalize pass can produce the final `Dead`/`Alive` value
    /// while births/deaths can be counted exactly once.
    pub fn from_transition(was_alive: bool, will_be_alive: bool) -> CellState {
        match (was_alive, will_be_alive) {
            (true, true) => CellState::Alive,
            (true, false) => CellState::Dying,
            (false, true) => CellState::Resurrecting,
            (false, false) => CellState::Dead,
        }
    }
}
