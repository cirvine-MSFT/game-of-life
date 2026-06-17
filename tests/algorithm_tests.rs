use game_of_life::{
    BlinkerBoardInitializer, BoardEditor, BoardInitializer, BoardUpdater, BoardView,
    CellCoordinate, CellState, CenteredBlinkerInitializer, DemoBoardInitializer,
    FullyAliveInitializer, InMemoryBoard, InPlaceTransitionalUpdater, RandomBoardInitializer,
    RandomBoardInitializerError,
};
use std::convert::Infallible;

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

fn assert_stabilizes_within(mut board: InMemoryBoard, max_generations: usize) {
    for generation in 1..=max_generations {
        let before = board.clone();
        board.advance_generation();

        if board == before {
            assert!(
                generation <= max_generations,
                "pattern should stabilize within {max_generations} generations"
            );
            return;
        }
    }

    panic!("pattern did not stabilize within {max_generations} generations");
}

fn live_cell_count(board: &InMemoryBoard) -> usize {
    let mut count = 0;
    for y in 0..board.height() {
        for x in 0..board.width() {
            if board.get(x, y) == CellState::Alive {
                count += 1;
            }
        }
    }
    count
}

mod normal_tests {
    use super::*;

    #[test]
    fn centered_blinker_initializer_seeds_middle_row() {
        let mut board = InMemoryBoard::new(5, 5);

        CenteredBlinkerInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&[".....", ".....", ".###.", ".....", "....."]);
        assert_eq!(board, expected);
    }

    #[test]
    fn blinker_board_initializer_seeds_middle_row() {
        let mut board = InMemoryBoard::new(5, 5);

        BlinkerBoardInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&[".....", ".....", ".###.", ".....", "....."]);
        assert_eq!(board, expected);
    }

    #[test]
    fn fully_alive_initializer_seeds_every_cell() {
        let mut board = InMemoryBoard::new(3, 2);

        FullyAliveInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&["###", "###"]);
        assert_eq!(board, expected);
    }

    #[test]
    fn fully_alive_board_larger_than_two_by_two_dies_quickly() {
        let mut board = InMemoryBoard::new(4, 4);

        FullyAliveInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");
        board.advance_generation();
        let expected_after_one = board_from_grid(&["#..#", "....", "....", "#..#"]);
        assert_eq!(board, expected_after_one);

        board.advance_generation();
        let expected_after_two = board_from_grid(&["....", "....", "....", "...."]);
        assert_eq!(board, expected_after_two);
    }

    #[test]
    fn fully_alive_two_by_two_board_is_stable() {
        let mut board = InMemoryBoard::new(2, 2);

        FullyAliveInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");
        let initial = board.clone();

        board.advance_generation();

        assert_eq!(board, initial);
    }

    #[test]
    fn demo_board_initializer_seeds_curated_ten_by_ten_pattern() {
        let mut board = InMemoryBoard::new(10, 10);

        DemoBoardInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&[
            "..........",
            "..........",
            ".....#.#..",
            "..#.##.#..",
            "......#...",
            "...##.....",
            "..##.#....",
            "...#......",
            "..........",
            "..........",
        ]);
        assert_eq!(board, expected);
    }

    #[test]
    fn demo_board_initializer_reaches_stability_within_twenty_generations() {
        let mut board = InMemoryBoard::new(10, 10);
        DemoBoardInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        for generation in 1..=20 {
            let before = board.clone();
            board.advance_generation();

            if board == before {
                let changed_generations = generation - 1;
                assert!(
                    changed_generations >= 5,
                    "demo pattern should visibly change before stabilizing"
                );
                assert!(
                    generation <= 20,
                    "demo pattern should stabilize within 20 generations"
                );
                return;
            }
        }

        panic!("demo pattern did not stabilize within 20 generations");
    }

    #[test]
    fn demo_board_initializer_repeats_independent_tiles_on_larger_boards() {
        let mut board = InMemoryBoard::new(24, 24);
        DemoBoardInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        assert_eq!(live_cell_count(&board), 52);
        assert_stabilizes_within(board, 20);
    }

    #[test]
    fn demo_board_initializer_stabilizes_across_representative_board_sizes() {
        for (width, height) in [
            (1, 1),
            (2, 2),
            (3, 5),
            (5, 3),
            (7, 9),
            (9, 7),
            (10, 10),
            (12, 12),
            (24, 10),
            (10, 24),
            (24, 24),
            (37, 25),
        ] {
            let mut board = InMemoryBoard::new(width, height);
            DemoBoardInitializer
                .initialize(&mut board)
                .expect("in-memory board initialization is infallible");

            assert_stabilizes_within(board, 20);
        }
    }

    #[test]
    fn grouped_read_returns_cells_in_requested_order() {
        let board = board_from_grid(&["#.", ".#"]);
        let coordinates = [
            CellCoordinate::new(1, 1),
            CellCoordinate::new(0, 0),
            CellCoordinate::new(1, 0),
        ];
        let mut states = vec![CellState::Resurrecting];

        board
            .read_cells(&coordinates, &mut states)
            .expect("in-memory board reads are infallible");

        assert_eq!(
            states,
            vec![CellState::Alive, CellState::Alive, CellState::Dead]
        );
    }

    #[test]
    fn in_place_transitional_updater_matches_blinker_behavior() {
        let mut board = board_from_grid(&["...", "###", "..."]);

        InPlaceTransitionalUpdater
            .advance_generation(&mut board)
            .expect("in-memory board updates are infallible");

        let expected = board_from_grid(&[".#.", ".#.", ".#."]);
        assert_eq!(board, expected);
    }

    #[test]
    fn random_board_initializer_is_reproducible_for_same_seed() {
        let initializer = RandomBoardInitializer::with_alive_cells_per_thousand(42, 500)
            .expect("valid random initializer density");
        let mut first = InMemoryBoard::new(6, 4);
        let mut second = InMemoryBoard::new(6, 4);

        initializer
            .initialize(&mut first)
            .expect("in-memory board initialization is infallible");
        initializer
            .initialize(&mut second)
            .expect("in-memory board initialization is infallible");

        assert_eq!(first, second);
    }

    #[test]
    fn random_board_initializer_replaces_existing_state() {
        let initializer = RandomBoardInitializer::with_alive_cells_per_thousand(42, 0)
            .expect("valid random initializer density");
        let mut board = board_from_grid(&["###", "###", "###"]);

        initializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&["...", "...", "..."]);
        assert_eq!(board, expected);
    }
}

mod rule_redesign_tests {
    use super::*;
    use game_of_life::CellRule;

    /// Reference implementation of standard Conway B3/S23, written from scratch
    /// against the rule's contract. Used to validate `InPlaceTransitionalUpdater`'s
    /// `CellRule` impl.
    fn reference_b3s23(currently_alive: bool, live_neighbors: usize) -> bool {
        matches!(
            (currently_alive, live_neighbors),
            (true, 2) | (true, 3) | (false, 3)
        )
    }

    #[test]
    fn in_place_transitional_updater_matches_b3s23_for_every_input() {
        let rule = InPlaceTransitionalUpdater;
        for &currently_alive in &[true, false] {
            for live_neighbors in 0..=8usize {
                assert_eq!(
                    rule.next_state(currently_alive, live_neighbors),
                    reference_b3s23(currently_alive, live_neighbors),
                    "rule disagrees with B3/S23 at \
                     (currently_alive={currently_alive}, live_neighbors={live_neighbors})"
                );
            }
        }
    }

    #[test]
    fn advance_with_rule_matches_legacy_advance_generation() {
        let patterns: &[&[&str]] = &[
            &["...", "###", "..."],
            &[".#.", ".#.", ".#."],
            &["##", "##"],
            &["#..", ".#.", "..#"],
            &["....", ".##.", ".##.", "...."],
            &["#####", ".....", "#...#", ".....", "#####"],
        ];

        for pattern in patterns {
            let mut via_rule = board_from_grid(pattern);
            let mut via_legacy = board_from_grid(pattern);

            via_rule
                .advance_with_rule(&InPlaceTransitionalUpdater)
                .expect("in-memory board updates are infallible");
            via_legacy.advance_generation();

            assert_eq!(
                via_rule, via_legacy,
                "advance_with_rule and advance_generation must agree for pattern {pattern:?}"
            );
        }
    }

    #[test]
    fn board_updater_advance_generation_delegates_to_rule_path() {
        // Confirm the old BoardUpdater entry point still works end-to-end via
        // the per-impl delegation to board.advance_with_rule.
        let mut board = board_from_grid(&["...", "###", "..."]);

        InPlaceTransitionalUpdater
            .advance_generation(&mut board)
            .expect("in-memory board updates are infallible");

        let expected = board_from_grid(&[".#.", ".#.", ".#."]);
        assert_eq!(board, expected);
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn edge_case_centered_blinker_initializer_handles_one_by_one_board() {
        let mut board = InMemoryBoard::new(1, 1);

        CenteredBlinkerInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&["#"]);
        assert_eq!(board, expected);
    }

    #[test]
    fn edge_case_centered_blinker_initializer_writes_only_in_bounds_cells() {
        let mut board = RecordingBoard::new(1, 1);

        CenteredBlinkerInitializer
            .initialize(&mut board)
            .expect("recording board initialization is infallible");

        assert_eq!(board.writes, vec![CellCoordinate::new(0, 0)]);
    }

    #[test]
    fn edge_case_demo_board_initializer_writes_only_in_bounds_cells() {
        let mut board = RecordingBoard::new(1, 1);

        DemoBoardInitializer
            .initialize(&mut board)
            .expect("recording board initialization is infallible");

        assert!(board
            .writes
            .iter()
            .all(|coordinate| coordinate.x < 1 && coordinate.y < 1));
    }

    #[test]
    fn edge_case_demo_board_initializer_uses_small_settling_motif_on_small_boards() {
        let mut board = InMemoryBoard::new(5, 5);
        DemoBoardInitializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&[".....", ".##..", ".#...", ".....", "....."]);
        assert_eq!(board, expected);
        assert_stabilizes_within(board, 20);
    }

    #[test]
    fn edge_case_grouped_read_returns_dead_for_out_of_bounds_cells() {
        let board = board_from_grid(&["#.", ".#"]);
        let coordinates = [
            CellCoordinate::new(2, 0),
            CellCoordinate::new(0, 2),
            CellCoordinate::new(1, 1),
        ];
        let mut states = Vec::new();

        board
            .read_cells(&coordinates, &mut states)
            .expect("in-memory board reads are infallible");

        assert_eq!(
            states,
            vec![CellState::Dead, CellState::Dead, CellState::Alive]
        );
    }

    #[test]
    fn edge_case_random_board_initializer_can_make_every_cell_alive() {
        let initializer = RandomBoardInitializer::with_alive_cells_per_thousand(42, 1000)
            .expect("valid random initializer density");
        let mut board = InMemoryBoard::new(3, 2);

        initializer
            .initialize(&mut board)
            .expect("in-memory board initialization is infallible");

        let expected = board_from_grid(&["###", "###"]);
        assert_eq!(board, expected);
    }

    #[test]
    fn edge_case_random_board_initializer_zero_density_uses_fill_fallback() {
        let initializer = RandomBoardInitializer::with_alive_cells_per_thousand(42, 0)
            .expect("valid random initializer density");
        let mut board = RecordingBoard::new(2, 2);

        initializer
            .initialize(&mut board)
            .expect("recording board initialization is infallible");

        assert_eq!(
            board.writes,
            vec![
                CellCoordinate::new(0, 0),
                CellCoordinate::new(1, 0),
                CellCoordinate::new(0, 1),
                CellCoordinate::new(1, 1),
            ]
        );
    }

    #[test]
    fn edge_case_random_board_initializer_full_density_uses_fill_fallback() {
        let initializer = RandomBoardInitializer::with_alive_cells_per_thousand(42, 1000)
            .expect("valid random initializer density");
        let mut board = RecordingBoard::new(2, 2);

        initializer
            .initialize(&mut board)
            .expect("recording board initialization is infallible");

        assert_eq!(
            board.writes,
            vec![
                CellCoordinate::new(0, 0),
                CellCoordinate::new(1, 0),
                CellCoordinate::new(0, 1),
                CellCoordinate::new(1, 1),
            ]
        );
    }

    #[test]
    fn edge_case_fully_alive_initializer_uses_generic_fill_fallback() {
        let mut board = RecordingBoard::new(2, 2);

        FullyAliveInitializer
            .initialize(&mut board)
            .expect("recording board initialization is infallible");

        assert_eq!(
            board.writes,
            vec![
                CellCoordinate::new(0, 0),
                CellCoordinate::new(1, 0),
                CellCoordinate::new(0, 1),
                CellCoordinate::new(1, 1),
            ]
        );
    }

    #[test]
    fn edge_case_algorithms_handle_empty_boards_without_writes() {
        let mut board = RecordingBoard::new(0, 0);

        CenteredBlinkerInitializer
            .initialize(&mut board)
            .expect("recording board initialization is infallible");
        RandomBoardInitializer::new(42)
            .initialize(&mut board)
            .expect("recording board initialization is infallible");
        InPlaceTransitionalUpdater
            .advance_generation(&mut board)
            .expect("recording board update is infallible");

        assert!(board.writes.is_empty());
    }
}

mod negative_tests {
    use super::*;

    #[test]
    fn negative_random_board_initializer_rejects_density_over_one_thousand() {
        let error = RandomBoardInitializer::with_alive_cells_per_thousand(42, 1001)
            .expect_err("density over 1000 should fail");

        assert_eq!(
            error,
            RandomBoardInitializerError::AliveCellsPerThousandTooLarge { value: 1001 }
        );
        assert!(error.to_string().contains("0 to 1000"));
    }

    #[test]
    fn negative_centered_blinker_initializer_propagates_board_write_errors() {
        let mut board = ErroringBoard::new(3, 1).fail_writes();

        let error = CenteredBlinkerInitializer
            .initialize(&mut board)
            .expect_err("board write error should propagate");

        assert_eq!(error, TestBoardError::Write);
    }

    #[test]
    fn negative_random_board_initializer_propagates_board_write_errors() {
        let mut board = ErroringBoard::new(1, 1).fail_writes();

        let error = RandomBoardInitializer::new(42)
            .initialize(&mut board)
            .expect_err("board write error should propagate");

        assert_eq!(error, TestBoardError::Write);
    }

    #[test]
    fn negative_random_board_initializer_fill_path_propagates_board_write_errors() {
        let mut board = ErroringBoard::new(1, 1).fail_writes();
        let initializer = RandomBoardInitializer::with_alive_cells_per_thousand(42, 1000)
            .expect("valid random initializer density");

        let error = initializer
            .initialize(&mut board)
            .expect_err("board write error should propagate");

        assert_eq!(error, TestBoardError::Write);
    }

    #[test]
    fn negative_fully_alive_initializer_propagates_board_write_errors() {
        let mut board = ErroringBoard::new(1, 1).fail_writes();

        let error = FullyAliveInitializer
            .initialize(&mut board)
            .expect_err("board write error should propagate");

        assert_eq!(error, TestBoardError::Write);
    }

    #[test]
    fn negative_in_place_transitional_updater_propagates_board_read_errors() {
        let mut board = ErroringBoard::new(1, 1).fail_reads();

        let error = InPlaceTransitionalUpdater
            .advance_generation(&mut board)
            .expect_err("board read error should propagate");

        assert_eq!(error, TestBoardError::Read);
    }

    #[test]
    fn negative_in_place_transitional_updater_propagates_board_write_errors() {
        let mut board = ErroringBoard::new(1, 1).fail_writes();

        let error = InPlaceTransitionalUpdater
            .advance_generation(&mut board)
            .expect_err("board write error should propagate");

        assert_eq!(error, TestBoardError::Write);
    }
}

struct RecordingBoard {
    width: usize,
    height: usize,
    writes: Vec<CellCoordinate>,
}

impl RecordingBoard {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            writes: Vec::new(),
        }
    }
}

impl BoardView for RecordingBoard {
    type Error = Infallible;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn cell_state(&self, _coordinate: CellCoordinate) -> Result<CellState, Self::Error> {
        Ok(CellState::Dead)
    }
}

impl BoardEditor for RecordingBoard {
    fn set_cell(
        &mut self,
        coordinate: CellCoordinate,
        _state: CellState,
    ) -> Result<(), Self::Error> {
        assert!(coordinate.x < self.width);
        assert!(coordinate.y < self.height);
        self.writes.push(coordinate);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestBoardError {
    Read,
    Write,
}

#[derive(Debug)]
struct ErroringBoard {
    width: usize,
    height: usize,
    fail_reads: bool,
    fail_writes: bool,
}

impl ErroringBoard {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            fail_reads: false,
            fail_writes: false,
        }
    }

    fn fail_reads(mut self) -> Self {
        self.fail_reads = true;
        self
    }

    fn fail_writes(mut self) -> Self {
        self.fail_writes = true;
        self
    }
}

impl BoardView for ErroringBoard {
    type Error = TestBoardError;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn cell_state(&self, _coordinate: CellCoordinate) -> Result<CellState, Self::Error> {
        if self.fail_reads {
            Err(TestBoardError::Read)
        } else {
            Ok(CellState::Dead)
        }
    }
}

impl BoardEditor for ErroringBoard {
    fn set_cell(
        &mut self,
        _coordinate: CellCoordinate,
        _state: CellState,
    ) -> Result<(), Self::Error> {
        if self.fail_writes {
            Err(TestBoardError::Write)
        } else {
            Ok(())
        }
    }
}
