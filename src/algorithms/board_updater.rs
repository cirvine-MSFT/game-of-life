use crate::board::BoardEditor;
use crate::stats::AdvanceOutcome;

/// Interface for algorithms that advance a board by one generation.
///
/// Concrete implementations own the update strategy: rule behavior, neighbor
/// definition, and runtime/space tradeoffs. The default implementation is
/// `InPlaceTransitionalUpdater`, which preserves the original single-buffer
/// Conway update behavior.
///
/// `advance_generation` returns an `AdvanceOutcome` summarizing how many cells
/// were born, died, and remain alive after the generation. Stats consumers
/// such as `RunStatisticsCollector` use these counts to accumulate run-level
/// statistics in O(1) per generation, without re-scanning the board.
pub trait BoardUpdater {
    fn advance_generation<B: BoardEditor + ?Sized>(
        &self,
        board: &mut B,
    ) -> Result<AdvanceOutcome, B::Error>;
}
