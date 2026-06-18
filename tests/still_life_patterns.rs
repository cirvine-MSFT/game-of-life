use game_of_life::{detect_known_still_life_patterns, CellState, InMemoryBoard, StillLifePattern};

fn board_from_grid(lines: &[&str]) -> InMemoryBoard {
    let height = lines.len();
    let width = if height > 0 { lines[0].len() } else { 0 };
    let mut board = InMemoryBoard::new(width, height);

    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let state = match ch {
                '#' => CellState::Alive,
                '.' | ' ' => CellState::Dead,
                _ => CellState::Dead,
            };
            board.set(x, y, state);
        }
    }

    board
}

mod normal_tests {
    use super::*;

    #[test]
    fn known_still_life_patterns_remain_stable_and_are_detected() {
        let cases = [
            (StillLifePattern::Block, board_from_grid(&["##", "##"])),
            (
                StillLifePattern::Beehive,
                board_from_grid(&[".##.", "#..#", ".##."]),
            ),
            (
                StillLifePattern::Loaf,
                board_from_grid(&[".##.", "#..#", ".#.#", "..#."]),
            ),
            (
                StillLifePattern::Boat,
                board_from_grid(&["##.", "#.#", ".#."]),
            ),
            (
                StillLifePattern::Tub,
                board_from_grid(&[".#.", "#.#", ".#."]),
            ),
        ];

        for (pattern, mut board) in cases {
            let initial = board.clone();
            let outcome = board.advance_generation();
            assert!(outcome.is_stable(), "{pattern:?} should produce no changes");
            assert_eq!(board, initial, "{pattern:?} should remain unchanged");

            let summary = detect_known_still_life_patterns(&board).unwrap();
            assert_eq!(summary.count(pattern), 1, "{pattern:?} should be detected");
            assert_eq!(summary.unknown_components, 0);
        }
    }

    #[test]
    fn detector_counts_multiple_separate_known_patterns() {
        let board = board_from_grid(&["##....", "##....", "......", "..#...", ".#.#..", "..#..."]);

        let summary = detect_known_still_life_patterns(&board).unwrap();

        assert_eq!(summary.count(StillLifePattern::Block), 1);
        assert_eq!(summary.count(StillLifePattern::Tub), 1);
        assert_eq!(summary.total_known_components(), 2);
        assert_eq!(summary.unknown_components, 0);
    }

    #[test]
    fn stable_unknown_components_are_reported_without_failing_detection() {
        let board = board_from_grid(&[".##.", "#..#", "#..#", ".##."]);

        let summary = detect_known_still_life_patterns(&board).unwrap();

        assert!(!summary.has_known_patterns());
        assert_eq!(summary.unknown_components, 1);
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn edge_case_rotated_and_reflected_boat_is_detected() {
        let board = board_from_grid(&["##.", "#.#", ".#."]);
        let reflected = board_from_grid(&[".##", "#.#", ".#."]);

        assert_eq!(
            detect_known_still_life_patterns(&board)
                .unwrap()
                .count(StillLifePattern::Boat),
            1
        );
        assert_eq!(
            detect_known_still_life_patterns(&reflected)
                .unwrap()
                .count(StillLifePattern::Boat),
            1
        );
    }
}

mod negative_tests {
    use super::*;

    #[test]
    fn oscillators_are_not_stable_after_one_generation() {
        let mut blinker = board_from_grid(&[".....", ".....", ".###.", ".....", "....."]);
        let mut toad = board_from_grid(&["....", ".###", "###.", "...."]);
        let mut beacon = board_from_grid(&["##..", "##..", "..##", "..##"]);

        assert!(!blinker.advance_generation().is_stable());
        assert!(!toad.advance_generation().is_stable());
        assert!(!beacon.advance_generation().is_stable());
    }
}
