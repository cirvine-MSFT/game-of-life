//! Run-level statistics accumulated over the course of a simulation.

use super::advance_outcome::AdvanceOutcome;
use super::iteration_series::IterationSeries;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub cycle: Option<CycleStatistics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleStatistics {
    pub start_generation: u64,
    pub detected_generation: u64,
    pub period: u64,
}

/// Coarse-grained outcome label written to the run record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    MaxIterations,
    Extinct,
    Stable,
    Cyclic,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::MaxIterations => "max_iterations",
            RunStatus::Extinct => "extinct",
            RunStatus::Stable => "stable",
            RunStatus::Cyclic => "cyclic",
        }
    }
}

pub fn terminal_status_for_outcome(outcome: AdvanceOutcome) -> Option<RunStatus> {
    if outcome.alive_count == 0 {
        Some(RunStatus::Extinct)
    } else if outcome.is_stable() {
        Some(RunStatus::Stable)
    } else {
        None
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
    alive_series: Vec<u64>,
    births_series: Vec<u64>,
    deaths_series: Vec<u64>,
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
            alive_series: vec![initial_alive_count],
            births_series: vec![0],
            deaths_series: vec![0],
        }
    }

    pub fn from_statistics(statistics: &RunStatistics) -> Self {
        // A finalized summary cannot reconstruct every generation; this
        // synthetic shape preserves endpoint/totals invariants only. Callers
        // that need exact persistence-series data must keep the original
        // collector.
        let series_len = statistics
            .iterations_run
            .try_into()
            .ok()
            .and_then(|iterations: usize| iterations.checked_add(1))
            .unwrap_or(1);
        let mut alive_series = vec![statistics.final_alive_count; series_len];
        alive_series[0] = statistics.initial_alive_count;
        let mut births_series = vec![0; series_len];
        let mut deaths_series = vec![0; series_len];
        if series_len > 1 {
            let last = series_len - 1;
            births_series[last] = statistics.total_births;
            deaths_series[last] = statistics.total_deaths;
        }
        Self {
            initial_alive_count: statistics.initial_alive_count,
            final_alive_count: statistics.final_alive_count,
            peak_alive_count: statistics.peak_alive_count,
            peak_alive_generation: statistics.peak_alive_generation,
            min_alive_count: statistics.min_alive_count,
            min_alive_generation: statistics.min_alive_generation,
            total_births: statistics.total_births,
            total_deaths: statistics.total_deaths,
            iterations_run: statistics.iterations_run,
            alive_series,
            births_series,
            deaths_series,
        }
    }

    pub fn record(&mut self, outcome: AdvanceOutcome) {
        self.iterations_run += 1;
        self.total_births += outcome.births;
        self.total_deaths += outcome.deaths;
        self.final_alive_count = outcome.alive_count;
        self.alive_series.push(outcome.alive_count);
        self.births_series.push(outcome.births);
        self.deaths_series.push(outcome.deaths);
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
        self.finalize_with_series(status, None).0
    }

    pub fn finalize_with_cycle(
        self,
        status: RunStatus,
        cycle: Option<CycleStatistics>,
    ) -> RunStatistics {
        self.finalize_with_series(status, cycle).0
    }

    pub fn finalize_with_series(
        self,
        status: RunStatus,
        cycle: Option<CycleStatistics>,
    ) -> (RunStatistics, IterationSeries) {
        let stats = RunStatistics {
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
            cycle,
        };
        let series = IterationSeries {
            alive: self.alive_series,
            births: self.births_series,
            deaths: self.deaths_series,
        };
        (stats, series)
    }
}
