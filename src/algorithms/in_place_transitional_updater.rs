use crate::board::{BoardEditor, CellCoordinate, CellState};
use crate::stats::AdvanceOutcome;

use super::BoardUpdater;

/// Conway updater using one board buffer and transitional cell states.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InPlaceTransitionalUpdater;

impl BoardUpdater for InPlaceTransitionalUpdater {
    fn advance_generation<B: BoardEditor + ?Sized>(
        &self,
        board: &mut B,
    ) -> Result<AdvanceOutcome, B::Error> {
        let mut neighbor_coordinates = Vec::with_capacity(8);
        let mut neighbor_states = Vec::with_capacity(8);

        for y in 0..board.height() {
            for x in 0..board.width() {
                let coordinate = CellCoordinate::new(x, y);
                let current = board.cell_state(coordinate)?;
                let live_neighbors = count_live_neighbors(
                    board,
                    coordinate,
                    &mut neighbor_coordinates,
                    &mut neighbor_states,
                )?;
                board.set_cell(coordinate, next_cell_state(current, live_neighbors))?;
            }
        }

        let mut births = 0u64;
        let mut deaths = 0u64;
        let mut alive_count = 0u64;
        for y in 0..board.height() {
            for x in 0..board.width() {
                let coordinate = CellCoordinate::new(x, y);
                let raw = board.cell_state(coordinate)?;
                match raw {
                    CellState::Resurrecting => births += 1,
                    CellState::Dying => deaths += 1,
                    _ => {}
                }
                let normalized = normalize_cell_state(raw);
                if matches!(normalized, CellState::Alive) {
                    alive_count += 1;
                }
                board.set_cell(coordinate, normalized)?;
            }
        }
        Ok(AdvanceOutcome::from_counts(births, deaths, alive_count))
    }
}

fn count_live_neighbors<B: BoardEditor + ?Sized>(
    board: &B,
    coordinate: CellCoordinate,
    neighbor_coordinates: &mut Vec<CellCoordinate>,
    neighbor_states: &mut Vec<CellState>,
) -> Result<usize, B::Error> {
    neighbor_coordinates.clear();

    for dy in [-1isize, 0, 1] {
        for dx in [-1isize, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }

            let Some(nx) = coordinate.x.checked_add_signed(dx) else {
                continue;
            };
            let Some(ny) = coordinate.y.checked_add_signed(dy) else {
                continue;
            };

            if nx >= board.width() || ny >= board.height() {
                continue;
            }

            neighbor_coordinates.push(CellCoordinate::new(nx, ny));
        }
    }

    board.read_cells(neighbor_coordinates.as_slice(), neighbor_states)?;
    Ok(neighbor_states
        .iter()
        .filter(|state| matches!(state, CellState::Alive | CellState::Dying))
        .count())
}

fn next_cell_state(current: CellState, live_neighbors: usize) -> CellState {
    match current {
        CellState::Alive => {
            if live_neighbors == 2 || live_neighbors == 3 {
                CellState::Alive
            } else {
                CellState::Dying
            }
        }
        CellState::Dead => {
            if live_neighbors == 3 {
                CellState::Resurrecting
            } else {
                CellState::Dead
            }
        }
        CellState::Dying => {
            if live_neighbors == 2 || live_neighbors == 3 {
                CellState::Alive
            } else {
                CellState::Dying
            }
        }
        CellState::Resurrecting => {
            if live_neighbors == 3 {
                CellState::Resurrecting
            } else {
                CellState::Dead
            }
        }
    }
}

fn normalize_cell_state(state: CellState) -> CellState {
    match state {
        CellState::Dying => CellState::Dead,
        CellState::Resurrecting => CellState::Alive,
        other => other,
    }
}
