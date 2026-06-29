//! Run statistics collection and the per-generation outcome model.

pub mod advance_outcome;
pub mod iteration_series;
pub mod run_statistics;

pub use advance_outcome::AdvanceOutcome;
pub use iteration_series::IterationSeries;
pub use run_statistics::{
    terminal_status_for_outcome, CycleStatistics, RunStatistics, RunStatisticsCollector,
};
