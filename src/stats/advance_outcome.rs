//! Per-generation outcome reported by the `BoardUpdater`.
//!
//! Letting the updater report births/deaths/alive_count after each generation
//! means the stats collector accumulates in O(1) per generation without
//! requiring a separate full board scan.

/// Outcome of advancing a board by exactly one generation.
///
/// Reported by `BoardUpdater::advance_generation` so the stats collector can
/// accumulate in O(1) per generation rather than re-scanning the board.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdvanceOutcome {
    pub births: u64,
    pub deaths: u64,
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
