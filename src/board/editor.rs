use super::{BoardView, CellCoordinate, CellState};
use crate::algorithms::CellRule;
use crate::stats::AdvanceOutcome;

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

    /// Advance the entire board by one generation under the given rule.
    ///
    /// The board controls iteration order, chunking, and any fusion of the
    /// transitional-state mark and normalize phases. Takes `&dyn CellRule`
    /// (not a generic) so `BoardEditor` remains object-safe.
    ///
    /// The default implementation runs the standard two-phase in-place update
    /// using the board's public surface — a mark pass that writes transitional
    /// states (`Dying` / `Resurrecting`) and a normalize pass that converts
    /// them back to `Dead` / `Alive`. Concrete board backends with chunked or
    /// streaming storage (e.g. `StreamingBoard`) override this method to fuse
    /// passes or amortize I/O.
    fn advance_with_rule(&mut self, rule: &dyn CellRule) -> Result<AdvanceOutcome, Self::Error> {
        default_advance_with_rule(self, rule)
    }
}

/// Default two-phase in-place generation update used by the `BoardEditor`
/// trait's default `advance_with_rule` impl.
///
/// Extracted as a free function so trait-object callers can borrow the
/// implementation if they need to bypass an override and the override can
/// still call back into the standard logic.
pub fn default_advance_with_rule<B: BoardEditor + ?Sized>(
    board: &mut B,
    rule: &dyn CellRule,
) -> Result<AdvanceOutcome, B::Error> {
    let height = board.height();
    let width = board.width();
    let mut neighbor_coordinates: Vec<CellCoordinate> = Vec::with_capacity(8);
    let mut neighbor_states: Vec<CellState> = Vec::with_capacity(8);

    // Mark pass: for each cell, count originally-alive neighbors via
    // CellState::is_originally_alive (so the count is unaffected by whether a
    // neighbor has already been overwritten with its transitional state), call
    // the rule, and write the transitional next state in place.
    for y in 0..height {
        for x in 0..width {
            let coordinate = CellCoordinate::new(x, y);
            let current = board.cell_state(coordinate)?;
            let was_alive = current.is_originally_alive();

            collect_in_bounds_neighbors(width, height, x, y, &mut neighbor_coordinates);
            board.read_cells(&neighbor_coordinates, &mut neighbor_states)?;
            let live_neighbors = neighbor_states
                .iter()
                .filter(|state| state.is_originally_alive())
                .count();

            let will_be_alive = rule.next_state(was_alive, live_neighbors);
            board.set_cell(
                coordinate,
                CellState::from_transition(was_alive, will_be_alive),
            )?;
        }
    }

    // Normalize pass: rewrite transitional cells to their final Dead/Alive
    // form and tally the per-generation outcome.
    let mut births = 0u64;
    let mut deaths = 0u64;
    let mut alive_count = 0u64;
    for y in 0..height {
        for x in 0..width {
            let coordinate = CellCoordinate::new(x, y);
            let raw = board.cell_state(coordinate)?;
            match raw {
                CellState::Resurrecting => births += 1,
                CellState::Dying => deaths += 1,
                _ => {}
            }
            let normalized = raw.normalized();
            if matches!(normalized, CellState::Alive) {
                alive_count += 1;
            }
            board.set_cell(coordinate, normalized)?;
        }
    }
    Ok(AdvanceOutcome::from_counts(births, deaths, alive_count))
}

fn collect_in_bounds_neighbors(
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    out: &mut Vec<CellCoordinate>,
) {
    out.clear();
    for dy in [-1isize, 0, 1] {
        for dx in [-1isize, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let Some(nx) = x.checked_add_signed(dx) else {
                continue;
            };
            let Some(ny) = y.checked_add_signed(dy) else {
                continue;
            };
            if nx >= width || ny >= height {
                continue;
            }
            out.push(CellCoordinate::new(nx, ny));
        }
    }
}
