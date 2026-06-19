//! Unit tests for `game_of_life::stats` (per-generation outcome + run-level
//! collector).

use game_of_life::stats::run_statistics::RunStatus;
use game_of_life::stats::{
    terminal_status_for_outcome, AdvanceOutcome, CycleStatistics, RunStatisticsCollector,
};

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
fn collector_can_finalize_stability_without_counting_confirmation() {
    let collector = RunStatisticsCollector::starting_from(4);
    let stats = collector.finalize(RunStatus::Stable);
    assert_eq!(stats.status, RunStatus::Stable);
    assert_eq!(stats.final_alive_count, 4);
    assert_eq!(stats.iterations_run, 0);
}

#[test]
fn collector_records_cycle_metadata() {
    let mut collector = RunStatisticsCollector::starting_from(3);
    collector.record(AdvanceOutcome::from_counts(2, 2, 3));
    collector.record(AdvanceOutcome::from_counts(2, 2, 3));
    let cycle = CycleStatistics {
        start_generation: 0,
        detected_generation: 2,
        period: 2,
    };
    let stats = collector.finalize_with_cycle(RunStatus::Cyclic, Some(cycle));
    assert_eq!(stats.status, RunStatus::Cyclic);
    assert_eq!(stats.iterations_run, 2);
    assert_eq!(stats.cycle, Some(cycle));
}

#[test]
fn collector_can_resume_from_finalized_statistics() {
    let mut collector = RunStatisticsCollector::starting_from(3);
    collector.record(AdvanceOutcome::from_counts(2, 2, 3));
    let stats = collector.finalize(RunStatus::MaxIterations);

    let mut resumed = RunStatisticsCollector::from_statistics(&stats);
    resumed.record(AdvanceOutcome::from_counts(2, 2, 3));
    let cycle = CycleStatistics {
        start_generation: 0,
        detected_generation: 2,
        period: 2,
    };
    let resumed_stats = resumed.finalize_with_cycle(RunStatus::Cyclic, Some(cycle));

    assert_eq!(resumed_stats.initial_alive_count, 3);
    assert_eq!(resumed_stats.final_alive_count, 3);
    assert_eq!(resumed_stats.total_births, 4);
    assert_eq!(resumed_stats.total_deaths, 4);
    assert_eq!(resumed_stats.iterations_run, 2);
    assert_eq!(resumed_stats.cycle, Some(cycle));
}

#[test]
fn terminal_status_prioritizes_extinction_over_stability() {
    let extinct = AdvanceOutcome::from_counts(0, 0, 0);
    assert_eq!(
        terminal_status_for_outcome(extinct),
        Some(RunStatus::Extinct)
    );

    let stable = AdvanceOutcome::from_counts(0, 0, 4);
    assert_eq!(terminal_status_for_outcome(stable), Some(RunStatus::Stable));

    let active = AdvanceOutcome::from_counts(1, 1, 4);
    assert_eq!(terminal_status_for_outcome(active), None);
}

#[test]
fn outcome_stability_depends_on_births_and_deaths() {
    assert!(AdvanceOutcome::from_counts(0, 0, 4).is_stable());
    assert!(!AdvanceOutcome::from_counts(1, 0, 5).is_stable());
    assert!(!AdvanceOutcome::from_counts(0, 1, 3).is_stable());
}

#[test]
fn status_string_representations() {
    assert_eq!(RunStatus::MaxIterations.as_str(), "max_iterations");
    assert_eq!(RunStatus::Extinct.as_str(), "extinct");
    assert_eq!(RunStatus::Stable.as_str(), "stable");
    assert_eq!(RunStatus::Cyclic.as_str(), "cyclic");
}
