use std::collections::{BTreeMap, VecDeque};

use crate::board::{BoardView, CellCoordinate, CellState};

type Point = (i32, i32);
type Transform = fn(Point) -> Point;
const MAX_CATALOG_PATTERN_CELLS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StillLifePattern {
    Block,
    Beehive,
    Loaf,
    Boat,
    Tub,
}

impl StillLifePattern {
    pub fn as_str(self) -> &'static str {
        match self {
            StillLifePattern::Block => "block",
            StillLifePattern::Beehive => "beehive",
            StillLifePattern::Loaf => "loaf",
            StillLifePattern::Boat => "boat",
            StillLifePattern::Tub => "tub",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StillLifePatternCount {
    pub pattern: StillLifePattern,
    pub count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StillLifePatternSummary {
    pub counts: Vec<StillLifePatternCount>,
    pub unknown_components: usize,
}

impl StillLifePatternSummary {
    pub fn count(&self, pattern: StillLifePattern) -> usize {
        self.counts
            .iter()
            .find(|entry| entry.pattern == pattern)
            .map_or(0, |entry| entry.count)
    }

    pub fn total_known_components(&self) -> usize {
        self.counts.iter().map(|entry| entry.count).sum()
    }

    pub fn has_known_patterns(&self) -> bool {
        self.total_known_components() > 0
    }
}

pub fn detect_known_still_life_patterns<B: BoardView>(
    board: &B,
) -> Result<StillLifePatternSummary, B::Error> {
    let width = board.width();
    let height = board.height();
    let cell_count = width
        .checked_mul(height)
        .expect("board dimensions exceed addressable cell capacity");
    let mut visited = vec![false; cell_count];
    let mut counts = BTreeMap::new();
    let mut unknown_components = 0usize;

    for y in 0..board.height() {
        for x in 0..board.width() {
            let coordinate = CellCoordinate::new(x, y);
            let index = cell_index(width, coordinate);
            if visited[index] {
                continue;
            }
            if !is_alive(board, coordinate)? {
                visited[index] = true;
                continue;
            }

            let component = collect_component(board, coordinate, width, height, &mut visited)?;
            if component.is_catalog_sized {
                if let Some(pattern) = classify_component(&component.cells) {
                    *counts.entry(pattern).or_insert(0) += 1;
                    continue;
                }
            }
            unknown_components += 1;
        }
    }

    Ok(StillLifePatternSummary {
        counts: counts
            .into_iter()
            .map(|(pattern, count)| StillLifePatternCount { pattern, count })
            .collect(),
        unknown_components,
    })
}

struct Component {
    cells: Vec<CellCoordinate>,
    is_catalog_sized: bool,
}

fn collect_component<B: BoardView>(
    board: &B,
    start: CellCoordinate,
    width: usize,
    height: usize,
    visited: &mut [bool],
) -> Result<Component, B::Error> {
    let mut cells = Vec::new();
    let mut is_catalog_sized = true;
    let mut queue = VecDeque::from([start]);
    visited[cell_index(width, start)] = true;

    while let Some(coordinate) = queue.pop_front() {
        if cells.len() < MAX_CATALOG_PATTERN_CELLS {
            cells.push(coordinate);
        } else {
            is_catalog_sized = false;
        }
        for neighbor in live_neighbors(coordinate, width, height) {
            let index = cell_index(width, neighbor);
            if visited[index] {
                continue;
            }
            visited[index] = true;
            if is_alive(board, neighbor)? {
                queue.push_back(neighbor);
            }
        }
    }

    Ok(Component {
        cells,
        is_catalog_sized,
    })
}

fn live_neighbors(
    coordinate: CellCoordinate,
    width: usize,
    height: usize,
) -> impl Iterator<Item = CellCoordinate> {
    let mut neighbors = Vec::with_capacity(8);
    for dy in [-1isize, 0, 1] {
        for dx in [-1isize, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let Some(x) = coordinate.x.checked_add_signed(dx) else {
                continue;
            };
            let Some(y) = coordinate.y.checked_add_signed(dy) else {
                continue;
            };
            if x >= width || y >= height {
                continue;
            }
            neighbors.push(CellCoordinate::new(x, y));
        }
    }
    neighbors.into_iter()
}

fn is_alive<B: BoardView>(board: &B, coordinate: CellCoordinate) -> Result<bool, B::Error> {
    Ok(matches!(
        board.cell_state(coordinate)?.normalized(),
        CellState::Alive
    ))
}

fn cell_index(width: usize, coordinate: CellCoordinate) -> usize {
    coordinate.y * width + coordinate.x
}

fn classify_component(component: &[CellCoordinate]) -> Option<StillLifePattern> {
    let signature = normalize_component(component);
    catalog()
        .iter()
        .find_map(|(pattern, shape)| shape_matches(&signature, shape).then_some(*pattern))
}

fn shape_matches(signature: &[Point], shape: &[Point]) -> bool {
    if signature.len() != shape.len() {
        return false;
    }
    transformed_variants(shape)
        .into_iter()
        .any(|variant| variant == signature)
}

fn normalize_component(component: &[CellCoordinate]) -> Vec<Point> {
    let min_x = component
        .iter()
        .map(|coordinate| coordinate.x)
        .min()
        .unwrap_or(0);
    let min_y = component
        .iter()
        .map(|coordinate| coordinate.y)
        .min()
        .unwrap_or(0);
    let mut normalized = component
        .iter()
        .map(|coordinate| ((coordinate.x - min_x) as i32, (coordinate.y - min_y) as i32))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized
}

fn transformed_variants(shape: &[Point]) -> Vec<Vec<Point>> {
    let transforms: [Transform; 8] = [
        |(x, y)| (x, y),
        |(x, y)| (x, -y),
        |(x, y)| (-x, y),
        |(x, y)| (-x, -y),
        |(x, y)| (y, x),
        |(x, y)| (y, -x),
        |(x, y)| (-y, x),
        |(x, y)| (-y, -x),
    ];

    let mut variants = Vec::with_capacity(transforms.len());
    for transform in transforms {
        let transformed = shape.iter().copied().map(transform).collect::<Vec<_>>();
        variants.push(normalize_points(transformed));
    }
    variants.sort_unstable();
    variants.dedup();
    variants
}

fn normalize_points(points: Vec<Point>) -> Vec<Point> {
    let min_x = points.iter().map(|(x, _)| *x).min().unwrap_or(0);
    let min_y = points.iter().map(|(_, y)| *y).min().unwrap_or(0);
    let mut normalized = points
        .into_iter()
        .map(|(x, y)| (x - min_x, y - min_y))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized
}

fn catalog() -> &'static [(StillLifePattern, &'static [Point])] {
    &[
        (StillLifePattern::Block, &[(0, 0), (1, 0), (0, 1), (1, 1)]),
        (
            StillLifePattern::Beehive,
            &[(1, 0), (2, 0), (0, 1), (3, 1), (1, 2), (2, 2)],
        ),
        (
            StillLifePattern::Loaf,
            &[(1, 0), (2, 0), (0, 1), (3, 1), (1, 2), (3, 2), (2, 3)],
        ),
        (
            StillLifePattern::Boat,
            &[(0, 0), (1, 0), (0, 1), (2, 1), (1, 2)],
        ),
        (StillLifePattern::Tub, &[(1, 0), (0, 1), (2, 1), (1, 2)]),
    ]
}
