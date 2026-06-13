//! Per-generation outcome reported by the `BoardUpdater`.
//!
//! Letting the updater report births/deaths/alive_count after each generation
//! means the stats collector accumulates in O(1) per generation without
//! requiring a separate full board scan.

/// Outcome of advancing a board by exactly one generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdvanceOutcome {
    /// Number of cells that transitioned from dead to alive in this
    /// generation.
    pub births: u64,
    /// Number of cells that transitioned from alive to dead in this
    /// generation.
    pub deaths: u64,
    /// Number of live cells on the board after this generation (i.e. the
    /// alive count in the normalized post-generation board).
    pub alive_count: u64,
}

impl AdvanceOutcome {
    pub fn from_counts(births: u64, deaths: u64, alive_count: u64) -> Self {
        Self {
            births,
            deaths,
            alive_count,
        }
    }
}
