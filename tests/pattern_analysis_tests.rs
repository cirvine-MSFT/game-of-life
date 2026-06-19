use game_of_life::{
    BoardSignature, CellState, InMemoryBoard, PatternAnalyzer, PatternBackend, PatternMatchDetails,
    PatternObservation,
};

fn board_from_grid(lines: &[&str]) -> InMemoryBoard {
    let height = lines.len();
    let width = if height > 0 { lines[0].len() } else { 0 };
    let mut board = InMemoryBoard::new(width, height);
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            board.set(
                x,
                y,
                if ch == '#' {
                    CellState::Alive
                } else {
                    CellState::Dead
                },
            );
        }
    }
    board
}

fn observe_generation_zero(analyzer: &mut PatternAnalyzer, board: &InMemoryBoard) {
    let signature = BoardSignature::from_view(board).unwrap();
    assert!(analyzer
        .observe(&PatternObservation::new(
            0,
            PatternBackend::InMemory,
            None,
            Some(&signature),
        ))
        .is_none());
}

fn assert_period_two_cycle(lines: &[&str]) {
    let mut board = board_from_grid(lines);
    let mut analyzer = PatternAnalyzer::in_memory_cycle_detection();
    observe_generation_zero(&mut analyzer, &board);

    let summary = board.advance_generation_with_signature();
    assert!(analyzer
        .observe(&PatternObservation::new(
            1,
            PatternBackend::InMemory,
            Some(summary.outcome),
            summary.signature.as_ref(),
        ))
        .is_none());

    let summary = board.advance_generation_with_signature();
    let detected = analyzer
        .observe(&PatternObservation::new(
            2,
            PatternBackend::InMemory,
            Some(summary.outcome),
            summary.signature.as_ref(),
        ))
        .expect("second oscillator phase should repeat generation 0");

    let PatternMatchDetails::Cycle(cycle) = detected.details;
    assert!(detected.terminal);
    assert_eq!(cycle.start_generation, 0);
    assert_eq!(cycle.detected_generation, 2);
    assert_eq!(cycle.period, 2);
}

#[test]
fn cycle_detector_reports_blinker_period_two() {
    assert_period_two_cycle(&[".....", ".....", ".###.", ".....", "....."]);
}

#[test]
fn cycle_detector_reports_toad_period_two() {
    assert_period_two_cycle(&["......", "......", "..###.", ".###..", "......", "......"]);
}

#[test]
fn cycle_detector_reports_beacon_period_two() {
    assert_period_two_cycle(&["......", ".##...", ".##...", "...##.", "...##.", "......"]);
}

#[test]
fn cycle_detector_ignores_non_repeating_transient() {
    let mut board = board_from_grid(&[
        "..........",
        "..#.......",
        "...#......",
        ".###......",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
    ]);
    let mut analyzer = PatternAnalyzer::in_memory_cycle_detection();
    observe_generation_zero(&mut analyzer, &board);

    for generation in 1..=4 {
        let summary = board.advance_generation_with_signature();
        assert!(
            analyzer
                .observe(&PatternObservation::new(
                    generation,
                    PatternBackend::InMemory,
                    Some(summary.outcome),
                    summary.signature.as_ref(),
                ))
                .is_none(),
            "glider should not repeat within four generations"
        );
    }
}
