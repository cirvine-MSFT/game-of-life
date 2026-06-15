use crate::algorithms::CellRule;
use crate::board::BoardEditor;
use crate::stats::AdvanceOutcome;

use super::BoardUpdater;

/// Conway B3/S23 rule, expressed as a [`CellRule`] and applied via the
/// board's [`BoardEditor::advance_with_rule`].
///
/// The original transitional-state two-pass algorithm now lives behind that
/// trait method — backends own the iteration so chunked / streaming variants
/// can fuse passes or amortize I/O while reusing the same rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InPlaceTransitionalUpdater;

impl CellRule for InPlaceTransitionalUpdater {
    fn next_state(&self, currently_alive: bool, live_neighbors: usize) -> bool {
        match (currently_alive, live_neighbors) {
            (true, 2) | (true, 3) => true,
            (true, _) => false,
            (false, 3) => true,
            (false, _) => false,
        }
    }
}

impl BoardUpdater for InPlaceTransitionalUpdater {
    fn advance_generation<B: BoardEditor + ?Sized>(
        &self,
        board: &mut B,
    ) -> Result<AdvanceOutcome, B::Error> {
        board.advance_with_rule(self)
    }
}
