//! Run-level statistics accumulated over the course of a simulation.

use super::advance_outcome::AdvanceOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunStatistics {
    pub initial_alive_count: u64,
    pub final_alive_count: u64,
    pub peak_alive_count: u64,
    pub peak_alive_generation: u64,
    pub min_alive_count: u64,
    pub min_alive_generation: u64,
    pub total_births: u64,
    pub total_deaths: u64,
    pub iterations_run: u64,
    pub status: RunStatus,
}

/// Coarse-grained outcome label written to the run record. Reserved future
/// values (`stable`, `cyclic`) are recognized by the reader but never emitted
/// by this version of the writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    MaxIterations,
    Extinct,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::MaxIterations => "max_iterations",
            RunStatus::Extinct => "extinct",
        }
    }
}

/// Collector for `RunStatistics`. Designed to be fed exactly one
/// `AdvanceOutcome` per generation in order; `finalize` consumes it and tags
/// the terminal status. Generation numbering starts at 1 for the first
/// `record` call — generation 0 is the initial board, captured at construction.
#[derive(Debug, Clone)]
pub struct RunStatisticsCollector {
    initial_alive_count: u64,
    final_alive_count: u64,
    peak_alive_count: u64,
    peak_alive_generation: u64,
    min_alive_count: u64,
    min_alive_generation: u64,
    total_births: u64,
    total_deaths: u64,
    iterations_run: u64,
}

impl RunStatisticsCollector {
    pub fn starting_from(initial_alive_count: u64) -> Self {
        Self {
            initial_alive_count,
            final_alive_count: initial_alive_count,
            peak_alive_count: initial_alive_count,
            peak_alive_generation: 0,
            min_alive_count: initial_alive_count,
            min_alive_generation: 0,
            total_births: 0,
            total_deaths: 0,
            iterations_run: 0,
        }
    }

    pub fn record(&mut self, outcome: AdvanceOutcome) {
        self.iterations_run += 1;
        self.total_births += outcome.births;
        self.total_deaths += outcome.deaths;
        self.final_alive_count = outcome.alive_count;
        if outcome.alive_count > self.peak_alive_count {
            self.peak_alive_count = outcome.alive_count;
            self.peak_alive_generation = self.iterations_run;
        }
        if outcome.alive_count < self.min_alive_count {
            self.min_alive_count = outcome.alive_count;
            self.min_alive_generation = self.iterations_run;
        }
    }

    pub fn iterations_run(&self) -> u64 {
        self.iterations_run
    }

    pub fn final_alive_count(&self) -> u64 {
        self.final_alive_count
    }

    pub fn finalize(self, status: RunStatus) -> RunStatistics {
        RunStatistics {
            initial_alive_count: self.initial_alive_count,
            final_alive_count: self.final_alive_count,
            peak_alive_count: self.peak_alive_count,
            peak_alive_generation: self.peak_alive_generation,
            min_alive_count: self.min_alive_count,
            min_alive_generation: self.min_alive_generation,
            total_births: self.total_births,
            total_deaths: self.total_deaths,
            iterations_run: self.iterations_run,
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_starts_with_initial_alive_count_as_peak_and_min() {
        let collector = RunStatisticsCollector::starting_from(7);
        let stats = collector.finalize(RunStatus::MaxIterations);
        assert_eq!(stats.initial_alive_count, 7);
        assert_eq!(stats.final_alive_count, 7);
        assert_eq!(stats.peak_alive_count, 7);
        assert_eq!(stats.min_alive_count, 7);
        assert_eq!(stats.peak_alive_generation, 0);
        assert_eq!(stats.min_alive_generation, 0);
        assert_eq!(stats.iterations_run, 0);
    }

    #[test]
    fn collector_tracks_peak_and_min_across_generations() {
        let mut collector = RunStatisticsCollector::starting_from(10);
        collector.record(AdvanceOutcome::from_counts(3, 1, 12)); // gen 1, peak
        collector.record(AdvanceOutcome::from_counts(0, 4, 8)); // gen 2
        collector.record(AdvanceOutcome::from_counts(1, 4, 5)); // gen 3, min
        collector.record(AdvanceOutcome::from_counts(2, 0, 7)); // gen 4
        let stats = collector.finalize(RunStatus::MaxIterations);
        assert_eq!(stats.peak_alive_count, 12);
        assert_eq!(stats.peak_alive_generation, 1);
        assert_eq!(stats.min_alive_count, 5);
        assert_eq!(stats.min_alive_generation, 3);
        assert_eq!(stats.total_births, 6);
        assert_eq!(stats.total_deaths, 9);
        assert_eq!(stats.iterations_run, 4);
        assert_eq!(stats.final_alive_count, 7);
    }

    #[test]
    fn collector_records_extinction() {
        let mut collector = RunStatisticsCollector::starting_from(3);
        collector.record(AdvanceOutcome::from_counts(0, 3, 0));
        let stats = collector.finalize(RunStatus::Extinct);
        assert_eq!(stats.status, RunStatus::Extinct);
        assert_eq!(stats.final_alive_count, 0);
        assert_eq!(stats.iterations_run, 1);
    }

    #[test]
    fn status_string_representations() {
        assert_eq!(RunStatus::MaxIterations.as_str(), "max_iterations");
        assert_eq!(RunStatus::Extinct.as_str(), "extinct");
    }
}
