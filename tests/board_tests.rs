use game_of_life::{CellState, InMemoryBoard, InMemoryBoardCreationError};

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
    fn still_life_block_remains_stable() {
        let mut board = board_from_grid(&["##", "##"]);

        let initial_state = board.clone();
        let outcome = board.advance_generation();

        assert!(outcome.is_stable());
        assert_eq!(board, initial_state, "Block should remain stable");
    }

    #[test]
    fn still_life_examples_produce_stable_outcomes() {
        let cases = [
            ("beehive", board_from_grid(&[".##.", "#..#", ".##."])),
            ("loaf", board_from_grid(&[".##.", "#..#", ".#.#", "..#."])),
            ("boat", board_from_grid(&["##.", "#.#", ".#."])),
            ("tub", board_from_grid(&[".#.", "#.#", ".#."])),
        ];

        for (name, mut board) in cases {
            let initial = board.clone();
            let outcome = board.advance_generation();

            assert!(outcome.is_stable(), "{name} should produce no changes");
            assert_eq!(board, initial, "{name} should remain unchanged");
        }
    }

    #[test]
    fn blinker_oscillator_returns_to_initial_state() {
        let mut board = board_from_grid(&["...", "###", "..."]);

        let initial = board.clone();

        board.advance_generation();
        let expected_after_1 = board_from_grid(&[".#.", ".#.", ".#."]);
        assert_eq!(
            board, expected_after_1,
            "After 1 generation, blinker should be vertical"
        );

        board.advance_generation();
        assert_eq!(
            board, initial,
            "After 2 generations, blinker should return to initial state"
        );
    }

    #[test]
    fn oscillators_are_not_stable_after_one_generation() {
        let mut blinker = board_from_grid(&[".....", ".....", ".###.", ".....", "....."]);
        let mut toad = board_from_grid(&["....", ".###", "###.", "...."]);
        let mut beacon = board_from_grid(&["##..", "##..", "..##", "..##"]);

        assert!(!blinker.advance_generation().is_stable());
        assert!(!toad.advance_generation().is_stable());
        assert!(!beacon.advance_generation().is_stable());
    }

    #[test]
    fn no_transitional_states_remain_after_generation() {
        let mut board = board_from_grid(&["###", "###", "###"]);

        board.advance_generation();

        for y in 0..board.height() {
            for x in 0..board.width() {
                let state = board.get(x, y);
                assert!(
                    state == CellState::Dead || state == CellState::Alive,
                    "Cell at ({x}, {y}) has transitional state: {state:?}"
                );
            }
        }
    }

    #[test]
    fn neighbor_counting_preserves_original_live_cells_during_mark_pass() {
        let mut board = board_from_grid(&["..#..", ".###.", ".....", ".....", "....."]);

        board.advance_generation();

        let expected = board_from_grid(&[".###.", ".###.", "..#..", ".....", "....."]);
        assert_eq!(
            board, expected,
            "Transitional states should preserve original neighbor count"
        );
    }

    #[test]
    fn in_memory_board_try_new_accepts_exact_memory_budget() {
        let requested_bytes =
            InMemoryBoard::allocation_bytes(3, 2).expect("small board should fit");

        let board = InMemoryBoard::try_new(3, 2, requested_bytes)
            .expect("exact memory budget should be accepted");

        assert_eq!(board.width(), 3);
        assert_eq!(board.height(), 2);
    }
}

mod cell_state_helpers {
    use super::*;

    #[test]
    fn is_originally_alive_treats_alive_and_dying_as_live() {
        assert!(CellState::Alive.is_originally_alive());
        assert!(CellState::Dying.is_originally_alive());
        assert!(!CellState::Dead.is_originally_alive());
        assert!(!CellState::Resurrecting.is_originally_alive());
    }

    #[test]
    fn normalized_converts_transitional_to_final() {
        assert_eq!(CellState::Alive.normalized(), CellState::Alive);
        assert_eq!(CellState::Dead.normalized(), CellState::Dead);
        assert_eq!(CellState::Dying.normalized(), CellState::Dead);
        assert_eq!(CellState::Resurrecting.normalized(), CellState::Alive);
    }

    #[test]
    fn from_transition_encodes_was_and_will_correctly() {
        assert_eq!(
            CellState::from_transition(true, true),
            CellState::Alive,
            "alive cell that stays alive is Alive"
        );
        assert_eq!(
            CellState::from_transition(true, false),
            CellState::Dying,
            "alive cell becoming dead is Dying"
        );
        assert_eq!(
            CellState::from_transition(false, true),
            CellState::Resurrecting,
            "dead cell becoming alive is Resurrecting"
        );
        assert_eq!(
            CellState::from_transition(false, false),
            CellState::Dead,
            "dead cell that stays dead is Dead"
        );
    }

    #[test]
    fn from_transition_round_trips_through_is_originally_alive() {
        for &was_alive in &[true, false] {
            for &will_be_alive in &[true, false] {
                let encoded = CellState::from_transition(was_alive, will_be_alive);
                assert_eq!(
                    encoded.is_originally_alive(),
                    was_alive,
                    "encoded transitional state must report its original liveness"
                );
                assert_eq!(
                    encoded.normalized() == CellState::Alive,
                    will_be_alive,
                    "normalizing the encoded state must yield the rule's verdict"
                );
            }
        }
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn edge_case_edge_cells_use_bounded_semantics() {
        let mut board = board_from_grid(&["...", "###", "..."]);

        board.advance_generation();

        let expected = board_from_grid(&[".#.", ".#.", ".#."]);
        assert_eq!(
            board, expected,
            "Edge cells should follow bounded board semantics"
        );
    }

    #[test]
    fn edge_case_corner_cell_with_no_neighbors_dies() {
        let mut board = board_from_grid(&["#  ", "   ", "   "]);

        board.advance_generation();

        let expected = board_from_grid(&["   ", "   ", "   "]);
        assert_eq!(board, expected, "Single corner cell should die");
    }

    #[test]
    fn edge_case_one_by_one_live_cell_dies_after_one_generation() {
        let mut board = board_from_grid(&["#"]);

        board.advance_generation();

        let expected = board_from_grid(&["."]);
        assert_eq!(board, expected, "Single live cell has no neighbors");
    }

    #[test]
    fn edge_case_out_of_bounds_get_and_set_are_safe() {
        let mut board = board_from_grid(&[".", "."]);

        assert_eq!(board.get(2, 0), CellState::Dead);
        assert_eq!(board.get(0, 2), CellState::Dead);

        board.set(2, 0, CellState::Alive);
        board.set(0, 2, CellState::Alive);

        let expected = board_from_grid(&[".", "."]);
        assert_eq!(board, expected, "Out-of-bounds set should not mutate board");
    }
}

mod negative_tests {
    use super::*;

    #[test]
    fn negative_in_memory_board_try_new_rejects_budget_overage() {
        let requested_bytes =
            InMemoryBoard::allocation_bytes(3, 2).expect("small board should fit");

        let error = InMemoryBoard::try_new(3, 2, requested_bytes - 1)
            .expect_err("memory budget below requested bytes should fail");

        assert_eq!(
            error,
            InMemoryBoardCreationError::MemoryBudgetExceeded {
                width: 3,
                height: 2,
                requested_memory_bytes: requested_bytes,
                max_memory_bytes: requested_bytes - 1,
            }
        );
        assert!(error.to_string().contains("configured max board memory"));
    }

    #[test]
    fn negative_in_memory_board_rejects_cell_count_overflow() {
        let error = InMemoryBoard::try_new(usize::MAX, 2, usize::MAX)
            .expect_err("cell count overflow should fail");

        assert_eq!(
            error,
            InMemoryBoardCreationError::CellCountOverflow {
                width: usize::MAX,
                height: 2,
            }
        );
        assert!(error.to_string().contains("width times height"));
    }

    #[test]
    fn negative_in_memory_board_rejects_unaddressable_allocation() {
        let width = isize::MAX as usize + 1;
        let error = InMemoryBoard::try_new(width, 1, usize::MAX)
            .expect_err("unaddressable allocation should fail");

        assert_eq!(
            error,
            InMemoryBoardCreationError::AllocationAddressSpaceExceeded {
                width,
                height: 1,
                requested_memory_bytes: width,
                max_addressable_bytes: isize::MAX as usize,
            }
        );
        assert!(error.to_string().contains("addressable allocation limit"));
    }
}
