use game_of_life::{Board, CellState};

fn board_from_grid(lines: &[&str]) -> Board {
    let height = lines.len();
    let width = if height > 0 { lines[0].len() } else { 0 };
    let mut board = Board::new(width, height);

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
        board.advance_generation();

        assert_eq!(board, initial_state, "Block should remain stable");
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
