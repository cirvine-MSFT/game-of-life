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
