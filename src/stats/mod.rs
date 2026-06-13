//! Run statistics collection and the per-generation outcome model.

pub mod advance_outcome;
pub mod run_statistics;

pub use advance_outcome::AdvanceOutcome;
pub use run_statistics::{RunStatistics, RunStatisticsCollector};
