use crate::board::BoardEditor;

/// Interface for algorithms that advance a board by one generation.
///
/// Concrete implementations own the update strategy: rule behavior, neighbor
/// definition, and runtime/space tradeoffs. The default implementation is
/// `InPlaceTransitionalUpdater`, which preserves the original single-buffer
/// Conway update behavior.
pub trait BoardUpdater {
    fn advance_generation<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error>;
}
