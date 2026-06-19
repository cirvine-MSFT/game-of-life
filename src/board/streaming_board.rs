//! File-backed streaming board for boards too large to fit in RAM.
//!
//! A `StreamingBoard` keeps only a small rectangular **chunk** of the
//! board resident in memory at any moment, paging the rest in and out
//! of a binary scratch file on disk. The chunk is composed from an
//! [`InMemoryBoard`] so all the existing cell storage and bounds-check
//! machinery is reused; this struct is responsible for *which* cells
//! are loaded and for the file I/O glue.
//!
//! # The owned / loaded distinction
//!
//! Every chunk position carries two rectangles, both in `usize` global
//! coordinates:
//!
//! - **Owned**: the cells the chunk will *update* at this position. The
//!   owned rectangles across all chunk positions form a partition of
//!   the board: every in-board cell belongs to exactly one chunk
//!   position's owned region.
//! - **Loaded**: owned cells plus a 1-cell halo on each non-board-edge
//!   side, used to satisfy 3×3 stencil reads without crossing into
//!   adjacent chunks.
//!
//! Out-of-board halo (e.g., the would-be left halo of a chunk whose
//! owned region starts at `x = 0`) is **never stored**. The streaming
//! board's own bounds check intercepts out-of-board reads and returns
//! `Dead`, so the chunk's `InMemoryBoard` never has to encode a virtual
//! halo.
//!
//! # Chunk dimensioning and the row-band fast path
//!
//! The owned dimensions are derived from `max_board_memory_bytes` using
//! `size_of::<CellState>()` per cell (the in-RAM cost — the 2-bit disk
//! packing is irrelevant to memory accounting).
//!
//! - **Row-band fast path**: when the memory budget is large enough to
//!   hold a chunk that spans the full board width (`owned_cols ==
//!   width`), no horizontal sliding ever happens and the chunk simply
//!   slides top-to-bottom. This is the common case and the path that
//!   gets the fused mark+normalize update treatment.
//! - **General 2D path**: when even one full row is too wide for the
//!   budget, the chunk shrinks horizontally and slides in both
//!   directions. Correct but slower; no fusion in v1.
//!
//! The streaming floor is `max(9 * RAM_CELL_BYTES + dirty_overhead, …)`
//! — enough RAM to hold the minimum 3×3 loaded chunk plus dirty row
//! tracking. Below that we error out with a suggested override.

use std::fmt;
use std::mem;
use std::path::{Path, PathBuf};

use crate::algorithms::CellRule;
use crate::board::{BoardEditor, BoardView, CellCoordinate, CellState, InMemoryBoard};
use crate::board::{
    BoardSignature, BoardSignatureAccumulator, BoardSignatureSource, GenerationSummary,
};
use crate::persistence::scratch::{ScratchFile, ScratchFileError};
use crate::stats::AdvanceOutcome;

/// Default working directory: the OS temp dir.
fn default_working_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Streaming board that backs cell storage with a scratch file on disk.
#[derive(Debug)]
pub struct StreamingBoard {
    // Backing scratch file holding all `width × height` cells (2 bits each
    // on disk). Random-access by row.
    scratch: ScratchFile,
    // Full board dimensions.
    board_width: usize,
    board_height: usize,
    // The in-memory chunk's storage. Sized to the maximum loaded extent
    // (`max_loaded_rows × max_loaded_cols`). The currently-loaded region
    // always occupies positions `(0..loaded_size.0, 0..loaded_size.1)`
    // inside this storage; positions outside that are irrelevant.
    chunk: InMemoryBoard,
    // Where the loaded region sits in global coordinates (top-left).
    loaded_origin: (usize, usize), // (y, x)
    loaded_size: (usize, usize),   // (rows, cols)
    // The owned sub-rectangle within the loaded rectangle (in global
    // coordinates).
    owned_origin: (usize, usize), // (y, x)
    owned_size: (usize, usize),   // (rows, cols)
    // Per *owned* row, has any cell been written since the last flush?
    dirty_rows: Vec<bool>,
    // The target owned dimensions used to compute chunk grid positions.
    // Edge chunks may have smaller actual owned sizes; interior chunks
    // get exactly these dimensions.
    target_owned_rows: usize,
    target_owned_cols: usize,
}

/// Errors returned by `StreamingBoard` setup.
#[derive(Debug)]
pub enum StreamingBoardCreationError {
    Scratch(ScratchFileError),
    InvalidDimensions {
        width: usize,
        height: usize,
    },
    InsufficientMemoryBudget {
        width: usize,
        height: usize,
        max_memory_bytes: usize,
        required_min_bytes: usize,
        suggested_min_bytes: usize,
    },
    WorkingDirUnavailable {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for StreamingBoardCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamingBoardCreationError::Scratch(e) => write!(f, "{e}"),
            StreamingBoardCreationError::InvalidDimensions { width, height } => write!(
                f,
                "Streaming board dimensions {width}x{height} must both be positive."
            ),
            StreamingBoardCreationError::InsufficientMemoryBudget {
                width,
                height,
                max_memory_bytes,
                required_min_bytes,
                suggested_min_bytes,
            } => write!(
                f,
                "Streaming board for {width}x{height} requires at least {required_min_bytes} bytes of working memory (the minimum 3x3 loaded chunk plus dirty-row tracking) but --max-board-memory is {max_memory_bytes} bytes. Try --max-board-memory {suggested_min_bytes}."
            ),
            StreamingBoardCreationError::WorkingDirUnavailable { path, source } => write!(
                f,
                "Could not prepare --working-dir '{}' for the scratch file: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for StreamingBoardCreationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StreamingBoardCreationError::Scratch(e) => Some(e),
            StreamingBoardCreationError::WorkingDirUnavailable { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<ScratchFileError> for StreamingBoardCreationError {
    fn from(e: ScratchFileError) -> Self {
        StreamingBoardCreationError::Scratch(e)
    }
}

/// Parameters used when constructing a `StreamingBoard`. Held in a struct
/// so callers don't have to track positional arguments and so optional
/// overrides (chunk dimensions) have clear names.
#[derive(Debug, Clone)]
pub struct StreamingBoardParams<'a> {
    pub width: usize,
    pub height: usize,
    pub max_board_memory_bytes: usize,
    pub working_dir: Option<&'a Path>,
    pub scratch_name_hint: &'a str,
    pub chunk_rows_override: Option<usize>,
    pub chunk_cols_override: Option<usize>,
}

impl StreamingBoard {
    /// Build a fresh streaming board. The backing scratch file is created
    /// under `working_dir` (or the OS temp dir if `None`) with a unique
    /// name derived from `scratch_name_hint`. Initial cell state is all
    /// `Dead`.
    pub fn new(params: StreamingBoardParams<'_>) -> Result<Self, StreamingBoardCreationError> {
        let StreamingBoardParams {
            width,
            height,
            max_board_memory_bytes,
            working_dir,
            scratch_name_hint,
            chunk_rows_override,
            chunk_cols_override,
        } = params;

        if width == 0 || height == 0 {
            return Err(StreamingBoardCreationError::InvalidDimensions { width, height });
        }

        let (target_owned_rows, target_owned_cols) = derive_chunk_dimensions(
            width,
            height,
            max_board_memory_bytes,
            chunk_rows_override,
            chunk_cols_override,
        )?;

        let max_loaded_rows = (target_owned_rows + 2).min(height);
        let max_loaded_cols = (target_owned_cols + 2).min(width);

        // Create the scratch file with a unique name. Auto-create the
        // working directory if it doesn't exist — users passing
        // `--working-dir /some/path` reasonably expect it to be created
        // for them (the OS temp dir is always there, but custom paths
        // often aren't). Anything that fails create_dir_all (permission
        // denied, name exists as a non-directory, etc.) surfaces with
        // its own clear OS error.
        let dir_owned;
        let dir = match working_dir {
            Some(d) => d,
            None => {
                dir_owned = default_working_dir();
                &dir_owned
            }
        };
        std::fs::create_dir_all(dir).map_err(|e| {
            StreamingBoardCreationError::WorkingDirUnavailable {
                path: dir.to_path_buf(),
                source: e,
            }
        })?;
        let scratch_path = dir.join(format!(
            "gol-scratch-{}-{}.bin",
            scratch_name_hint,
            unique_suffix()
        ));
        let scratch = ScratchFile::create(&scratch_path, width as u64, height as u64)?;

        // Allocate the chunk InMemoryBoard at max loaded extent.
        let chunk = InMemoryBoard::new(max_loaded_cols, max_loaded_rows);

        let mut board = StreamingBoard {
            scratch,
            board_width: width,
            board_height: height,
            chunk,
            loaded_origin: (0, 0),
            loaded_size: (0, 0),
            owned_origin: (0, 0),
            owned_size: (0, 0),
            dirty_rows: Vec::new(),
            target_owned_rows,
            target_owned_cols,
        };
        // Seed the chunk to its top-left position so the board is
        // immediately usable. Scratch was zero-initialized by
        // `ScratchFile::create`, so no I/O is needed beyond reading the
        // freshly-zeroed cells.
        board.slide_to_chunk_containing(0, 0)?;
        Ok(board)
    }

    /// Path of the backing scratch file. Useful for tests and for the
    /// lifecycle layer.
    pub fn scratch_path(&self) -> &Path {
        self.scratch.path()
    }

    /// Returns the target owned chunk dimensions (rows, cols). These are
    /// what the chunk uses for non-edge positions; edge chunks may be
    /// smaller due to board-boundary clamping.
    pub fn target_owned_chunk(&self) -> (usize, usize) {
        (self.target_owned_rows, self.target_owned_cols)
    }

    /// True if this board can use the row-band fast path
    /// (`owned_cols == width`).
    pub fn is_row_band_fast_path(&self) -> bool {
        self.target_owned_cols == self.board_width
    }

    /// Flush any pending dirty rows in the current chunk to the scratch
    /// file. Also ensures the underlying file is synced.
    pub fn flush(&mut self) -> Result<(), ScratchFileError> {
        self.flush_dirty_rows()?;
        self.scratch.flush()
    }

    /// Read a single cell, sliding the chunk if necessary so the cell
    /// is loaded. Mirrors `cell_state` but takes `&mut self` so it can
    /// trigger a slide (whereas `cell_state` is `&self`-bound by the
    /// `BoardView` trait contract and returns an error for out-of-chunk
    /// in-board reads).
    ///
    /// Used by snapshot / dump tooling and by tests that need to read
    /// arbitrary cells without writing them.
    pub fn peek_cell(&mut self, coordinate: CellCoordinate) -> Result<CellState, ScratchFileError> {
        if coordinate.x >= self.board_width || coordinate.y >= self.board_height {
            return Ok(CellState::Dead);
        }
        if !self.is_in_loaded(coordinate.x, coordinate.y) {
            self.slide_to_chunk_containing(coordinate.x, coordinate.y)?;
        }
        let local_x = coordinate.x - self.loaded_origin.1;
        let local_y = coordinate.y - self.loaded_origin.0;
        Ok(self.chunk.get(local_x, local_y))
    }

    // ---- chunk management ----

    /// Translate `(x, y)` to the chunk position (owned origin) that
    /// contains it, using the target owned dimensions to lay out the
    /// chunk grid.
    fn chunk_origin_for(&self, x: usize, y: usize) -> (usize, usize) {
        let y_grid = (y / self.target_owned_rows) * self.target_owned_rows;
        let x_grid = (x / self.target_owned_cols) * self.target_owned_cols;
        (y_grid, x_grid)
    }

    /// True if `(x, y)` is currently inside the loaded rectangle (and
    /// thus directly addressable in `self.chunk`).
    fn is_in_loaded(&self, x: usize, y: usize) -> bool {
        y >= self.loaded_origin.0
            && y < self.loaded_origin.0 + self.loaded_size.0
            && x >= self.loaded_origin.1
            && x < self.loaded_origin.1 + self.loaded_size.1
    }

    /// True if `(x, y)` is currently inside the owned rectangle (and
    /// thus a cell this chunk position will/may write).
    fn is_in_owned(&self, x: usize, y: usize) -> bool {
        y >= self.owned_origin.0
            && y < self.owned_origin.0 + self.owned_size.0
            && x >= self.owned_origin.1
            && x < self.owned_origin.1 + self.owned_size.1
    }

    /// Ensure the chunk currently has cell `(x, y)` in its owned region
    /// (sliding if necessary). After this call, `(x, y)` is both loaded
    /// and inside owned, so reads and writes are safe.
    fn ensure_owned(&mut self, x: usize, y: usize) -> Result<(), ScratchFileError> {
        if self.is_in_owned(x, y) {
            return Ok(());
        }
        self.slide_to_chunk_containing(x, y)
    }

    /// Compute the loaded rectangle that surrounds the given owned
    /// rectangle, clamped at board edges.
    fn loaded_rect_for_owned(
        &self,
        owned_origin: (usize, usize),
        owned_size: (usize, usize),
    ) -> ((usize, usize), (usize, usize)) {
        let y_start = owned_origin.0.saturating_sub(1);
        let x_start = owned_origin.1.saturating_sub(1);
        let y_end = (owned_origin.0 + owned_size.0 + 1).min(self.board_height);
        let x_end = (owned_origin.1 + owned_size.1 + 1).min(self.board_width);
        ((y_start, x_start), (y_end - y_start, x_end - x_start))
    }

    /// Compute the owned size for a chunk grid position (clamped at
    /// the bottom and right board edges).
    fn owned_size_at(&self, owned_y_start: usize, owned_x_start: usize) -> (usize, usize) {
        let rows = self
            .target_owned_rows
            .min(self.board_height - owned_y_start);
        let cols = self.target_owned_cols.min(self.board_width - owned_x_start);
        (rows, cols)
    }

    /// Flush dirty rows of the current chunk to scratch, then slide so
    /// that the chunk position containing `(x, y)` is loaded.
    fn slide_to_chunk_containing(&mut self, x: usize, y: usize) -> Result<(), ScratchFileError> {
        // 1. Flush whatever's currently dirty.
        self.flush_dirty_rows()?;

        // 2. Compute the new owned + loaded rectangles.
        let (new_owned_y, new_owned_x) = self.chunk_origin_for(x, y);
        let new_owned_size = self.owned_size_at(new_owned_y, new_owned_x);
        let (new_loaded_origin, new_loaded_size) =
            self.loaded_rect_for_owned((new_owned_y, new_owned_x), new_owned_size);

        // 3. Update metadata.
        self.owned_origin = (new_owned_y, new_owned_x);
        self.owned_size = new_owned_size;
        self.loaded_origin = new_loaded_origin;
        self.loaded_size = new_loaded_size;
        self.dirty_rows.clear();
        self.dirty_rows.resize(new_owned_size.0, false);

        // 4. Load the new loaded rectangle from scratch.
        self.load_chunk_from_scratch()?;
        Ok(())
    }

    /// Read the loaded rectangle from the scratch file into the chunk's
    /// `InMemoryBoard`, one row at a time.
    fn load_chunk_from_scratch(&mut self) -> Result<(), ScratchFileError> {
        let (loaded_y, loaded_x) = self.loaded_origin;
        let (loaded_rows, loaded_cols) = self.loaded_size;
        let mut row_buf: Vec<CellState> = Vec::with_capacity(loaded_cols);
        for local_y in 0..loaded_rows {
            let global_y = loaded_y + local_y;
            self.scratch.read_row_range(
                global_y as u64,
                loaded_x as u64,
                (loaded_x + loaded_cols) as u64,
                &mut row_buf,
            )?;
            for (local_x, state) in row_buf.iter().enumerate() {
                self.chunk.set(local_x, local_y, *state);
            }
        }
        Ok(())
    }

    /// Write dirty rows of the current chunk to the scratch file. Only
    /// the owned sub-region is flushed; halo cells are read-only.
    fn flush_dirty_rows(&mut self) -> Result<(), ScratchFileError> {
        if self.dirty_rows.is_empty() || self.owned_size.0 == 0 || self.owned_size.1 == 0 {
            return Ok(());
        }
        let (owned_y, owned_x) = self.owned_origin;
        let (owned_rows, owned_cols) = self.owned_size;
        let (loaded_y, loaded_x) = self.loaded_origin;
        // Owned region in chunk-local coords.
        let owned_local_y_start = owned_y - loaded_y;
        let owned_local_x_start = owned_x - loaded_x;
        let mut row_buf: Vec<CellState> = Vec::with_capacity(owned_cols);

        for owned_row_idx in 0..owned_rows {
            if !self.dirty_rows[owned_row_idx] {
                continue;
            }
            let local_y = owned_local_y_start + owned_row_idx;
            row_buf.clear();
            for x_offset in 0..owned_cols {
                let local_x = owned_local_x_start + x_offset;
                row_buf.push(self.chunk.get(local_x, local_y));
            }
            let global_y = owned_y + owned_row_idx;
            self.scratch
                .write_row_range(global_y as u64, owned_x as u64, &row_buf)?;
        }
        for d in &mut self.dirty_rows {
            *d = false;
        }
        Ok(())
    }
}

/// Compute the working-set RAM cost of a chunk with the given owned
/// dimensions: storage for the loaded `(owned_rows + 2) × (owned_cols + 2)`
/// rectangle plus per-owned-row dirty bits.
fn ram_cost_for(owned_rows: usize, owned_cols: usize) -> usize {
    let loaded_rows = owned_rows + 2;
    let loaded_cols = owned_cols + 2;
    loaded_rows
        .saturating_mul(loaded_cols)
        .saturating_mul(mem::size_of::<CellState>())
        .saturating_add(owned_rows.saturating_mul(mem::size_of::<bool>()))
}

/// Derive target owned chunk dimensions from the memory budget and
/// optional CLI overrides. Width-first: prefer the row-band fast path
/// whenever the budget can afford a chunk that spans the full board
/// width.
pub fn derive_chunk_dimensions(
    board_width: usize,
    board_height: usize,
    max_memory_bytes: usize,
    chunk_rows_override: Option<usize>,
    chunk_cols_override: Option<usize>,
) -> Result<(usize, usize), StreamingBoardCreationError> {
    // The minimum chunk is 1x1 owned, 3x3 loaded, plus 1 bool for the
    // dirty bit on the single owned row.
    let min_required_bytes = ram_cost_for(1, 1);
    if max_memory_bytes < min_required_bytes {
        return Err(StreamingBoardCreationError::InsufficientMemoryBudget {
            width: board_width,
            height: board_height,
            max_memory_bytes,
            required_min_bytes: min_required_bytes,
            suggested_min_bytes: min_required_bytes,
        });
    }

    if let (Some(rows), Some(cols)) = (chunk_rows_override, chunk_cols_override) {
        let rows = rows.max(1).min(board_height);
        let cols = cols.max(1).min(board_width);
        return Ok((rows, cols));
    }

    // Width-first: try to fit a full-width row-band chunk (chunk_cols == width).
    // For row-band with `r` owned rows, the loaded chunk is `(r+2) × (width+2)`
    // (or `(r+2) × width` at the right edge if owned spans full width — same
    // formula since width is already max). Actually loaded_cols = width + 2 if
    // owned_cols == width and width < board_width — but owned_cols == width
    // means we ARE the full board width, so loaded_cols = width too (no halo
    // because we're at both left and right board edges). Use the precise
    // ram_cost_for formula.
    if let Some(row_band_rows) =
        best_owned_rows_for_full_width(board_width, board_height, max_memory_bytes)
    {
        let rows = row_band_rows.max(1).min(board_height);
        let cols = board_width;
        return Ok((rows, cols));
    }

    // General 2D path: fix owned_rows = 1 (minimum), grow owned_cols.
    let (rows, cols) = derive_2d_chunk(board_width, board_height, max_memory_bytes);
    Ok((rows.min(board_height), cols.min(board_width)))
}

/// If the budget can afford a row-band chunk (owned_cols == board_width),
/// return the largest number of owned rows that fits. Otherwise return
/// None.
fn best_owned_rows_for_full_width(
    board_width: usize,
    board_height: usize,
    max_memory_bytes: usize,
) -> Option<usize> {
    // RAM cost of an owned `r × width` chunk = (r+2) * (width+2) * cell_bytes + r * bool_bytes.
    // But "width + 2" double-counts if width already equals board_width: in
    // that case the chunk doesn't need horizontal halo. ram_cost_for assumes
    // both halos are present, which is conservative — using it here means we
    // may report "doesn't fit" when in reality it would. Since this just
    // affects the auto-derivation heuristic (the actual chunk allocation
    // matches `ram_cost_for`), being conservative is safe.
    let cell_bytes = mem::size_of::<CellState>();
    let bool_bytes = mem::size_of::<bool>();
    // Binary search the largest r in [1, board_height] that fits.
    let mut lo = 1usize;
    let mut hi = board_height;
    let mut best: Option<usize> = None;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let loaded_rows = mid + 2;
        let loaded_cols = board_width + 2;
        let cost = loaded_rows
            .saturating_mul(loaded_cols)
            .saturating_mul(cell_bytes)
            .saturating_add(mid.saturating_mul(bool_bytes));
        if cost <= max_memory_bytes {
            best = Some(mid);
            if mid == hi {
                break;
            }
            lo = mid + 1;
        } else if mid == 1 {
            return None;
        } else {
            hi = mid - 1;
        }
    }
    best
}

/// General-2D derivation: target_owned_rows = 1, grow cols.
fn derive_2d_chunk(
    board_width: usize,
    _board_height: usize,
    max_memory_bytes: usize,
) -> (usize, usize) {
    let cell_bytes = mem::size_of::<CellState>();
    let bool_bytes = mem::size_of::<bool>();
    let owned_rows = 1usize;
    let dirty_overhead = owned_rows * bool_bytes;
    let available_for_cells = max_memory_bytes.saturating_sub(dirty_overhead);
    // For owned 1 x c, loaded = (1+2) * (c+2) cells.
    // available_for_cells >= 3 * (c+2) * cell_bytes
    // c <= available_for_cells / (3 * cell_bytes) - 2
    let denom = 3usize.saturating_mul(cell_bytes);
    let max_c_plus_2 = available_for_cells / denom.max(1);
    let mut owned_cols = max_c_plus_2.saturating_sub(2);
    if owned_cols == 0 {
        owned_cols = 1;
    }
    owned_cols = owned_cols.min(board_width);
    (owned_rows, owned_cols)
}

/// Generate a short suffix that is *guaranteed unique within this process*
/// (via a monotonic counter) and very likely unique across processes
/// (via time + pid mixing). The counter is the safety net: on platforms
/// with coarse `SystemTime` resolution, back-to-back calls in a tight
/// loop can otherwise return identical timestamps and collide.
fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    let mixed = nanos
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(pid)
        .wrapping_add(counter);
    format!("{mixed:016x}")
}

impl BoardView for StreamingBoard {
    type Error = ScratchFileError;

    fn width(&self) -> usize {
        self.board_width
    }

    fn height(&self) -> usize {
        self.board_height
    }

    fn cell_state(&self, coordinate: CellCoordinate) -> Result<CellState, Self::Error> {
        // Out of board => Dead (the streaming board's bounds check, not
        // the chunk's; this is critical so the chunk's artificial edges
        // are never confused with the real board edges).
        if coordinate.x >= self.board_width || coordinate.y >= self.board_height {
            return Ok(CellState::Dead);
        }
        // In loaded region => answer from chunk.
        if self.is_in_loaded(coordinate.x, coordinate.y) {
            let local_x = coordinate.x - self.loaded_origin.1;
            let local_y = coordinate.y - self.loaded_origin.0;
            return Ok(self.chunk.get(local_x, local_y));
        }
        // In board but not in current chunk => need a slide. But
        // BoardView::cell_state takes &self, so we can't mutate. Return
        // an explicit error that signals "callers must use the
        // mutating path for cross-chunk reads, or pre-slide via
        // ensure_owned." For our own internal usage (advance_with_rule)
        // we always pre-slide.
        Err(ScratchFileError::OutOfBounds {
            detail: format!(
                "cell_state ({}, {}) is in-board but outside the currently-loaded chunk; \
                use ensure_owned (via set_cell or advance_with_rule) before reading",
                coordinate.x, coordinate.y
            ),
        })
    }
}

impl BoardEditor for StreamingBoard {
    fn set_cell(
        &mut self,
        coordinate: CellCoordinate,
        state: CellState,
    ) -> Result<(), Self::Error> {
        // Out of board => ignored, matching InMemoryBoard's set_cell.
        if coordinate.x >= self.board_width || coordinate.y >= self.board_height {
            return Ok(());
        }
        self.ensure_owned(coordinate.x, coordinate.y)?;
        let local_x = coordinate.x - self.loaded_origin.1;
        let local_y = coordinate.y - self.loaded_origin.0;
        self.chunk.set(local_x, local_y, state);
        let owned_row_idx = coordinate.y - self.owned_origin.0;
        self.dirty_rows[owned_row_idx] = true;
        Ok(())
    }

    /// Override the default `advance_with_rule` with chunked iteration so
    /// we don't trip the streaming board's "cell_state can't slide on
    /// &self" restriction. Two separate chunked passes (mark, then
    /// normalize) for v1 — row-band fusion is a follow-up optimization.
    fn advance_with_rule(&mut self, rule: &dyn CellRule) -> Result<AdvanceOutcome, Self::Error> {
        // Mark pass over all chunk positions in row-major order.
        self.iter_chunks_for_mark(rule)?;
        // Normalize pass over all chunk positions; tallies outcome.
        let outcome = self.iter_chunks_for_normalize()?;
        Ok(outcome)
    }

    fn advance_with_rule_and_signature(
        &mut self,
        rule: &dyn CellRule,
    ) -> Result<GenerationSummary, Self::Error> {
        Ok(GenerationSummary::new(self.advance_with_rule(rule)?, None))
    }
}

impl BoardSignatureSource for StreamingBoard {
    type Error = ScratchFileError;

    fn board_signature(&mut self) -> Result<BoardSignature, Self::Error> {
        let mut accumulator = BoardSignatureAccumulator::new(self.board_width, self.board_height);
        for y in 0..self.board_height {
            for x in 0..self.board_width {
                let coordinate = CellCoordinate::new(x, y);
                let state = self.peek_cell(coordinate)?;
                accumulator.observe(coordinate, state);
            }
        }
        Ok(accumulator.finish())
    }
}

impl StreamingBoard {
    fn iter_chunks_for_mark(&mut self, rule: &dyn CellRule) -> Result<(), ScratchFileError> {
        let mut y = 0usize;
        while y < self.board_height {
            let mut x = 0usize;
            while x < self.board_width {
                self.slide_to_chunk_containing(x, y)?;
                self.mark_current_chunk(rule);
                x = self.owned_origin.1 + self.owned_size.1;
            }
            y = self.owned_origin.0 + self.owned_size.0;
        }
        // Flush the last chunk's dirty rows; subsequent slides already
        // flush as part of their setup, but there is no next slide.
        self.flush_dirty_rows()?;
        Ok(())
    }

    fn iter_chunks_for_normalize(&mut self) -> Result<AdvanceOutcome, ScratchFileError> {
        let mut births = 0u64;
        let mut deaths = 0u64;
        let mut alive_count = 0u64;
        let mut y = 0usize;
        while y < self.board_height {
            let mut x = 0usize;
            while x < self.board_width {
                self.slide_to_chunk_containing(x, y)?;
                let partial = self.normalize_current_chunk();
                births += partial.births;
                deaths += partial.deaths;
                alive_count += partial.alive_count;
                x = self.owned_origin.1 + self.owned_size.1;
            }
            y = self.owned_origin.0 + self.owned_size.0;
        }
        self.flush_dirty_rows()?;
        Ok(AdvanceOutcome::from_counts(births, deaths, alive_count))
    }

    /// Mark pass for cells in the current chunk's owned rectangle.
    /// Reads neighbors from the loaded rectangle (which includes any
    /// transitional cells written by previously-processed adjacent
    /// chunks — `is_originally_alive` handles both original and
    /// transitional values).
    fn mark_current_chunk(&mut self, rule: &dyn CellRule) {
        let (owned_y, owned_x) = self.owned_origin;
        let (owned_rows, owned_cols) = self.owned_size;
        for dy in 0..owned_rows {
            for dx in 0..owned_cols {
                let gx = owned_x + dx;
                let gy = owned_y + dy;
                let current = self.chunk_cell_at_global(gx, gy);
                let was_alive = current.is_originally_alive();

                let mut live = 0usize;
                for ndy in [-1isize, 0, 1] {
                    for ndx in [-1isize, 0, 1] {
                        if ndx == 0 && ndy == 0 {
                            continue;
                        }
                        let Some(nx) = gx.checked_add_signed(ndx) else {
                            continue;
                        };
                        let Some(ny) = gy.checked_add_signed(ndy) else {
                            continue;
                        };
                        if nx >= self.board_width || ny >= self.board_height {
                            continue;
                        }
                        if self.chunk_cell_at_global(nx, ny).is_originally_alive() {
                            live += 1;
                        }
                    }
                }

                let will_be_alive = rule.next_state(was_alive, live);
                let local_x = gx - self.loaded_origin.1;
                let local_y = gy - self.loaded_origin.0;
                self.chunk.set(
                    local_x,
                    local_y,
                    CellState::from_transition(was_alive, will_be_alive),
                );
                self.dirty_rows[dy] = true;
            }
        }
    }

    /// Normalize the current chunk's owned cells (transitional ->
    /// Dead/Alive) and tally per-cell birth/death/alive counts.
    fn normalize_current_chunk(&mut self) -> AdvanceOutcome {
        let (owned_y, owned_x) = self.owned_origin;
        let (owned_rows, owned_cols) = self.owned_size;
        let mut births = 0u64;
        let mut deaths = 0u64;
        let mut alive_count = 0u64;
        for dy in 0..owned_rows {
            for dx in 0..owned_cols {
                let local_x = (owned_x + dx) - self.loaded_origin.1;
                let local_y = (owned_y + dy) - self.loaded_origin.0;
                let raw = self.chunk.get(local_x, local_y);
                match raw {
                    CellState::Resurrecting => births += 1,
                    CellState::Dying => deaths += 1,
                    _ => {}
                }
                let normalized = raw.normalized();
                if matches!(normalized, CellState::Alive) {
                    alive_count += 1;
                }
                self.chunk.set(local_x, local_y, normalized);
                self.dirty_rows[dy] = true;
            }
        }
        AdvanceOutcome::from_counts(births, deaths, alive_count)
    }

    /// Get a cell by global coordinates from the currently-loaded chunk.
    /// Out-of-board => Dead (the streaming board's bounds check).
    /// Asserts (via debug_assert) that any in-board read is inside the
    /// loaded rectangle — callers in `advance_with_rule` always ensure
    /// this by sliding before reading.
    fn chunk_cell_at_global(&self, gx: usize, gy: usize) -> CellState {
        if gx >= self.board_width || gy >= self.board_height {
            return CellState::Dead;
        }
        debug_assert!(
            self.is_in_loaded(gx, gy),
            "chunk_cell_at_global called for ({gx}, {gy}) outside loaded rect"
        );
        let local_x = gx - self.loaded_origin.1;
        let local_y = gy - self.loaded_origin.0;
        self.chunk.get(local_x, local_y)
    }
}

// Dummy impl so callers that want to drop the board without an explicit
// flush still get the lifecycle exit they expect. Per the plan, scratch
// files are left on disk for crashes; we don't attempt fancy Drop logic.
impl Drop for StreamingBoard {
    fn drop(&mut self) {
        // Best-effort flush. Errors are swallowed because Drop can't
        // return them; the lifecycle layer should call `flush()`
        // explicitly before drop for guaranteed durability.
        let _ = self.flush_dirty_rows();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_dimensions_picks_row_band_when_budget_allows() {
        // 4-wide board, generous budget => full-width row-band.
        let (rows, cols) =
            derive_chunk_dimensions(4, 100, 1024, None, None).expect("derive should succeed");
        assert_eq!(cols, 4, "should use full width as row-band");
        assert!(rows >= 1);
    }

    #[test]
    fn derive_dimensions_falls_back_to_2d_when_budget_too_small_for_row_band() {
        // Very tight budget on a wide board forces general-2D.
        let (rows, cols) = derive_chunk_dimensions(
            1000,
            1000,
            ram_cost_for(1, 1) + 4, // just over the minimum
            None,
            None,
        )
        .expect("minimum budget should succeed");
        assert_eq!(rows, 1);
        assert!(cols < 1000);
        assert!(cols >= 1);
    }

    #[test]
    fn derive_dimensions_rejects_budget_below_min() {
        let err = derive_chunk_dimensions(10, 10, 1, None, None)
            .expect_err("below-min budget should reject");
        assert!(matches!(
            err,
            StreamingBoardCreationError::InsufficientMemoryBudget { .. }
        ));
    }

    #[test]
    fn derive_dimensions_honors_overrides() {
        let (rows, cols) =
            derive_chunk_dimensions(100, 100, usize::MAX, Some(5), Some(7)).expect("derive");
        assert_eq!((rows, cols), (5, 7));
    }
}
