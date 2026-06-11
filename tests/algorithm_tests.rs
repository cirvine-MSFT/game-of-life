use game_of_life::{
    BoardEditor, BoardInitializer, BoardUpdater, BoardView, CellCoordinate, CellState,
    CenteredBlinkerInitializer, InMemoryBoard, InPlaceTransitionalUpdater, RandomBoardInitializer,
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
