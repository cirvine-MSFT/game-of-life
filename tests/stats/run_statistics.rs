//! Unit tests for `game_of_life::stats` (per-generation outcome + run-level
//! collector).

use game_of_life::stats::run_statistics::RunStatus;
use game_of_life::stats::{AdvanceOutcome, RunStatisticsCollector};

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
