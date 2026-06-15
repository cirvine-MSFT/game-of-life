//! Pure cell-update rules.
//!
//! A [`CellRule`] is a stateless decision function: given whether a cell is
//! currently alive and how many of its neighbors are currently alive, return
//! whether the cell is alive next generation.
//!
//! Rules see only the two-state world (`bool` in / `bool` out). The four-state
//! `CellState` enum (`Dead`, `Alive`, `Dying`, `Resurrecting`) is a private
//! implementation detail used by board backends to sequence the two-phase
//! transitional-state algorithm; rules describe the *what*, backends describe
//! the *how*. Using `bool` makes the contract type-system-enforced — a rule
//! cannot accidentally return `Dying` or `Resurrecting`.
//!
//! Boards apply rules via [`BoardEditor::advance_with_rule`].
//!
//! [`BoardEditor::advance_with_rule`]: crate::board::BoardEditor::advance_with_rule

/// Pure rule deciding the next state of a single cell.
///
/// Implementations must be deterministic and side-effect-free. They receive
/// `currently_alive` (the cell's state at the start of the current
/// generation) and `live_neighbors` (the count of currently-alive neighbor
/// cells, in [`0`, `8`] for the standard 8-neighbor stencil), and return the
/// cell's state at the end of the generation.
pub trait CellRule {
    fn next_state(&self, currently_alive: bool, live_neighbors: usize) -> bool;
}
