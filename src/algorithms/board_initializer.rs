use crate::board::BoardEditor;

/// Interface for algorithms that initialize a board with a starting state.
///
/// Concrete implementations decide which cells become alive or dead. Examples
/// include deterministic patterns, random seeded patterns, or future file-based
/// pattern loaders. The trait is intentionally separate from concrete board
/// implementations so initializers can target any storage backend that
/// implements `BoardEditor`.
pub trait BoardInitializer {
    fn initialize<B: BoardEditor + ?Sized>(&self, board: &mut B) -> Result<(), B::Error>;
}
