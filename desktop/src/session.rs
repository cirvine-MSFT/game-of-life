//! `RunSession` — the simulation state machine the IPC layer drives.
//!
//! Critical-section discipline: every public method acquires the inner
//! `Mutex` for the **minimum** time needed to read/mutate. The play and
//! jump workers call `advance_one` in a tight loop and check
//! `take_cancel` between each call, so cancellation latency stays
//! bounded by a single generation rather than a whole `advance(N)`
//! batch. This is the fix for the critic's CRITICAL finding that a
//! batched `advance` would freeze the UI.
//!
//! History decimation: alive-count time series is uncapped during a
//! run, but if it grows past `HISTORY_CAP` the older entries are
//! decimated in place (every-other) so the Recharts panel never has
//! to render an arbitrarily large series.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{Mutex, MutexGuard};

use game_of_life::stats::run_statistics::RunStatus;
use game_of_life::{
    BlinkerBoardInitializer, BoardInitializer, CellState, DemoBoardInitializer,
    FullyAliveInitializer, InMemoryBoard, InMemoryBoardCreationError, RandomBoardInitializer,
    RandomBoardInitializerError, RunStatistics, RunStatisticsCollector,
};

use crate::ipc_types::{
    AdvanceTick, BoardPayload, CellEdit, InitialSource, IpcCellState, IpcRunStatistics,
    IpcRunStatus, Mode, PatternName, SessionInfo,
};

/// Upper bound on the alive-count history before in-place decimation.
/// At 60 gps a run can grow this in ~28 minutes; well past that point a
/// straight time series is no longer useful for visual inspection.
const HISTORY_CAP: usize = 100_000;

/// Default memory budget for the board allocation, matching the CLI's
/// `DEFAULT_MAX_BOARD_MEMORY_BYTES` (64 MiB). Reusing the same default
/// keeps semantics consistent between the two front-ends.
pub const DEFAULT_MAX_BOARD_MEMORY_BYTES: usize = 64 * 1024 * 1024;

/// Everything inside `RunSession` that the cancel flag does not cover.
struct SessionData {
    mode: Mode,
    width: u32,
    height: u32,
    board: Option<InMemoryBoard>,
    initial_snapshot: Option<Vec<u8>>,
    iteration: u64,
    max_iterations: u64,
    alive_history: Vec<u64>,
    stats: Option<RunStatisticsCollector>,
    final_stats: Option<RunStatistics>,
    save_path: Option<PathBuf>,
    dirty: bool,
    jump_target: Option<u64>,
    shadow_buf: Vec<u8>,
}

impl SessionData {
    fn new() -> Self {
        Self {
            mode: Mode::Setup,
            width: 0,
            height: 0,
            board: None,
            initial_snapshot: None,
            iteration: 0,
            max_iterations: 0,
            alive_history: Vec::new(),
            stats: None,
            final_stats: None,
            save_path: None,
            dirty: false,
            jump_target: None,
            shadow_buf: Vec::new(),
        }
    }

    /// Re-reads the board into the shadow buffer. Callers must hold the
    /// mutex. Cheap relative to `advance_generation` so we don't try to
    /// stay lazy here.
    fn refresh_shadow(&mut self) {
        let Some(board) = self.board.as_ref() else {
            self.shadow_buf.clear();
            return;
        };
        let cells = self.width as usize * self.height as usize;
        if self.shadow_buf.len() != cells {
            self.shadow_buf = vec![0u8; cells];
        }
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                self.shadow_buf[y * self.width as usize + x] =
                    IpcCellState::from_core(board.get(x, y)) as u8;
            }
        }
    }

    fn push_history(&mut self, alive: u64) {
        self.alive_history.push(alive);
        if self.alive_history.len() > HISTORY_CAP {
            // Halve the series in place. Doubling the effective time spacing
            // keeps the chart's macro-shape intact while bounding memory.
            let decimated: Vec<u64> = self.alive_history.iter().step_by(2).copied().collect();
            self.alive_history = decimated;
        }
    }
}

/// Top-level session handle. Cheap to clone via `Arc` from command handlers.
pub struct RunSession {
    inner: Mutex<SessionData>,
    cancel: AtomicBool,
}

impl Default for RunSession {
    fn default() -> Self {
        Self::new()
    }
}

impl RunSession {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SessionData::new()),
            cancel: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, SessionData> {
        self.inner.lock()
    }

    /// Snapshot of session metadata for the frontend toolbar / status bar.
    pub fn info(&self) -> SessionInfo {
        let data = self.lock();
        SessionInfo {
            mode: data.mode,
            iteration: data.iteration,
            width: data.width,
            height: data.height,
            max_iterations: data.max_iterations,
            save_path: data.save_path.as_ref().map(|p| p.display().to_string()),
            dirty: data.dirty,
            completed: data.final_stats.is_some(),
            jump_target: data.jump_target,
            status: data.final_stats.as_ref().map(|s| IpcRunStatus::from_core(s.status)),
        }
    }

    /// Returns the current cells + dims + iteration as a wire payload.
    pub fn board_payload(&self) -> BoardPayload {
        let mut data = self.lock();
        data.refresh_shadow();
        BoardPayload::from_bytes(data.width, data.height, data.iteration, &data.shadow_buf)
    }

    /// Cumulative alive counts the stats panel chart consumes. Returns a
    /// clone so callers can drop the lock immediately.
    pub fn alive_history(&self) -> Vec<u64> {
        self.lock().alive_history.clone()
    }

    /// Final run statistics once the simulation has terminated.
    pub fn final_stats(&self) -> Option<IpcRunStatistics> {
        self.lock().final_stats.as_ref().map(IpcRunStatistics::from)
    }

    /// Sets up a fresh run in Setup mode. Discards any prior in-flight run.
    /// Memory budget defaults to `DEFAULT_MAX_BOARD_MEMORY_BYTES` when
    /// `max_memory_bytes` is `None`.
    pub fn create_run(
        &self,
        width: u32,
        height: u32,
        source: InitialSource,
        max_iterations: u64,
        max_memory_bytes: Option<usize>,
    ) -> Result<(), SessionError> {
        if width == 0 || height == 0 {
            return Err(SessionError::ZeroDimension);
        }
        let budget = max_memory_bytes.unwrap_or(DEFAULT_MAX_BOARD_MEMORY_BYTES);
        let mut board = InMemoryBoard::try_new(width as usize, height as usize, budget)?;

        match source {
            InitialSource::Empty => {}
            InitialSource::Pattern(name) => apply_pattern_to(&mut board, name),
            InitialSource::Random {
                seed,
                alive_cells_per_thousand,
            } => {
                let init = RandomBoardInitializer::with_alive_cells_per_thousand(
                    seed,
                    alive_cells_per_thousand,
                )?;
                init.initialize(&mut board)
                    .expect("InMemoryBoard editor is infallible");
            }
        }

        let mut data = self.lock();
        data.mode = Mode::Setup;
        data.width = width;
        data.height = height;
        data.board = Some(board);
        data.initial_snapshot = None;
        data.iteration = 0;
        data.max_iterations = max_iterations;
        data.alive_history = Vec::new();
        data.stats = None;
        data.final_stats = None;
        data.save_path = None;
        data.dirty = false;
        data.jump_target = None;
        data.shadow_buf = Vec::new();
        self.cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn set_cell(&self, x: u32, y: u32, alive: bool) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let (w, h) = (data.width, data.height);
        if x >= w || y >= h {
            return Err(SessionError::OutOfBounds { x, y });
        }
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        let state = if alive { CellState::Alive } else { CellState::Dead };
        board.set(x as usize, y as usize, state);
        data.dirty = true;
        Ok(())
    }

    pub fn paint_cells(&self, edits: &[CellEdit]) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let (w, h) = (data.width, data.height);
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        for edit in edits {
            if edit.x >= w || edit.y >= h {
                return Err(SessionError::OutOfBounds {
                    x: edit.x,
                    y: edit.y,
                });
            }
            let state = if edit.alive {
                CellState::Alive
            } else {
                CellState::Dead
            };
            board.set(edit.x as usize, edit.y as usize, state);
        }
        if !edits.is_empty() {
            data.dirty = true;
        }
        Ok(())
    }

    pub fn apply_pattern(&self, pattern: PatternName) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        apply_pattern_to(board, pattern);
        data.dirty = true;
        Ok(())
    }

    pub fn randomize(
        &self,
        seed: u64,
        alive_cells_per_thousand: u16,
    ) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        let init = RandomBoardInitializer::with_alive_cells_per_thousand(
            seed,
            alive_cells_per_thousand,
        )?;
        init.initialize(board)
            .expect("InMemoryBoard editor is infallible");
        data.dirty = true;
        Ok(())
    }

    pub fn clear_board(&self) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let (w, h) = (data.width as usize, data.height as usize);
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        for y in 0..h {
            for x in 0..w {
                board.set(x, y, CellState::Dead);
            }
        }
        data.dirty = true;
        Ok(())
    }

    /// Locks the current Setup board in as the initial snapshot and enters
    /// the Paused running state. Subsequent `restart` calls restore from
    /// this snapshot.
    pub fn start_run(&self) -> Result<(), SessionError> {
        let mut data = self.lock();
        if !matches!(data.mode, Mode::Setup) {
            return Err(SessionError::WrongMode {
                current: data.mode,
                required: "Setup",
            });
        }
        let board = data.board.as_ref().ok_or(SessionError::NoBoard)?;
        let initial_snapshot: Vec<u8> = collect_cells(board);
        let initial_alive_count = count_alive(&initial_snapshot);
        data.initial_snapshot = Some(initial_snapshot);
        data.iteration = 0;
        data.alive_history = vec![initial_alive_count];
        data.stats = Some(RunStatisticsCollector::starting_from(initial_alive_count));
        data.final_stats = None;
        data.mode = Mode::Paused;
        data.jump_target = None;
        self.cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Restores the board to its initial snapshot, resets iteration and
    /// stats history. Caller must be in a Running mode.
    pub fn restart(&self) -> Result<(), SessionError> {
        let mut data = self.lock();
        if !matches!(data.mode, Mode::Paused | Mode::Playing | Mode::JumpingTo) {
            return Err(SessionError::WrongMode {
                current: data.mode,
                required: "Running",
            });
        }
        let snapshot = data
            .initial_snapshot
            .clone()
            .ok_or(SessionError::NoInitialSnapshot)?;
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        write_cells(board, &snapshot);
        let initial_alive_count = count_alive(&snapshot);
        data.iteration = 0;
        data.alive_history = vec![initial_alive_count];
        data.stats = Some(RunStatisticsCollector::starting_from(initial_alive_count));
        data.final_stats = None;
        data.mode = Mode::Paused;
        data.jump_target = None;
        self.cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Exits Running mode back into Setup, dropping the run record but
    /// keeping the current board (so the user can keep painting).
    pub fn edit_board(&self) -> Result<(), SessionError> {
        let mut data = self.lock();
        if matches!(data.mode, Mode::Setup) {
            return Ok(());
        }
        data.mode = Mode::Setup;
        data.initial_snapshot = None;
        data.iteration = 0;
        data.alive_history = Vec::new();
        data.stats = None;
        data.final_stats = None;
        data.jump_target = None;
        self.cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Raises `max_iterations`. Useful for Run-to-N where the user wants
    /// the simulation to keep going past its original cap.
    pub fn extend_max_iterations(&self, new_total: u64) -> Result<(), SessionError> {
        let mut data = self.lock();
        if new_total < data.iteration {
            return Err(SessionError::InvalidMaxIterations { new_total });
        }
        data.max_iterations = new_total;
        // Re-opening room past the cap removes any prior terminal state.
        if data
            .final_stats
            .as_ref()
            .map(|s| matches!(s.status, RunStatus::MaxIterations))
            .unwrap_or(false)
            && new_total > data.iteration
        {
            data.final_stats = None;
        }
        Ok(())
    }

    pub fn set_mode(&self, mode: Mode) {
        let mut data = self.lock();
        data.mode = mode;
    }

    pub fn set_jump_target(&self, target: Option<u64>) {
        let mut data = self.lock();
        data.jump_target = target;
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Workers call this once per iteration; true means a stop was requested
    /// since the last `clear_cancel`.
    pub fn cancel_requested(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn clear_cancel(&self) {
        self.cancel.store(false, Ordering::SeqCst);
    }

    pub fn set_save_path(&self, path: Option<PathBuf>) {
        let mut data = self.lock();
        data.save_path = path;
    }

    pub fn mark_dirty(&self, dirty: bool) {
        self.lock().dirty = dirty;
    }

    /// Advances by exactly one generation. Returns the per-generation
    /// tick payload; the caller is responsible for emitting events.
    /// Sets terminal state and stops the run if max_iterations or
    /// extinction is reached.
    pub fn advance_one(&self) -> Result<AdvanceTick, SessionError> {
        let mut data = self.lock();
        if data.final_stats.is_some() {
            return Err(SessionError::RunCompleted);
        }
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        let outcome = board.advance_generation();

        let total_cells = data.width as u64 * data.height as u64;
        data.iteration += 1;

        if let Some(stats) = data.stats.as_mut() {
            stats.record(outcome);
        }
        data.push_history(outcome.alive_count);

        let tick = AdvanceTick::from_outcome(data.iteration, total_cells, outcome);

        // Terminal-state detection: extinction takes priority over reaching
        // the iteration ceiling so the more interesting status wins on the
        // edge case where both happen on the same generation.
        let extinct = outcome.alive_count == 0;
        let hit_cap = data.iteration >= data.max_iterations;
        if extinct || hit_cap {
            let status = if extinct {
                RunStatus::Extinct
            } else {
                RunStatus::MaxIterations
            };
            if let Some(stats) = data.stats.take() {
                data.final_stats = Some(stats.finalize(status));
            }
            data.mode = Mode::Paused;
            self.cancel.store(false, Ordering::SeqCst);
        }

        Ok(tick)
    }
}

fn require_setup(data: &SessionData) -> Result<(), SessionError> {
    if matches!(data.mode, Mode::Setup) {
        Ok(())
    } else {
        Err(SessionError::WrongMode {
            current: data.mode,
            required: "Setup",
        })
    }
}

fn apply_pattern_to(board: &mut InMemoryBoard, pattern: PatternName) {
    let result = match pattern {
        PatternName::Demo => DemoBoardInitializer.initialize(board),
        PatternName::Blinker => BlinkerBoardInitializer.initialize(board),
        PatternName::FullyAlive => FullyAliveInitializer.initialize(board),
    };
    result.expect("InMemoryBoard editor is infallible");
}

fn collect_cells(board: &InMemoryBoard) -> Vec<u8> {
    let mut out = Vec::with_capacity(board.width() * board.height());
    for y in 0..board.height() {
        for x in 0..board.width() {
            out.push(IpcCellState::from_core(board.get(x, y)) as u8);
        }
    }
    out
}

fn write_cells(board: &mut InMemoryBoard, bytes: &[u8]) {
    let w = board.width();
    for y in 0..board.height() {
        for x in 0..w {
            let alive = bytes.get(y * w + x).copied().unwrap_or(0) != 0;
            let state = if alive { CellState::Alive } else { CellState::Dead };
            board.set(x, y, state);
        }
    }
}

fn count_alive(bytes: &[u8]) -> u64 {
    bytes.iter().filter(|&&b| b != 0).count() as u64
}

/// Errors any session method can surface across the IPC boundary.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("operation requires {required} mode but session is in {current:?}")]
    WrongMode {
        current: Mode,
        required: &'static str,
    },
    #[error("no board has been created in this session")]
    NoBoard,
    #[error("session has no initial snapshot — start_run was never called")]
    NoInitialSnapshot,
    #[error("cell ({x}, {y}) is out of bounds for the current board")]
    OutOfBounds { x: u32, y: u32 },
    #[error("max_iterations ({new_total}) must be greater than or equal to current iteration")]
    InvalidMaxIterations { new_total: u64 },
    #[error("the run has already completed; restart or edit_board to continue")]
    RunCompleted,
    #[error("board width and height must be greater than zero")]
    ZeroDimension,
    #[error(transparent)]
    Allocation(#[from] InMemoryBoardCreationError),
    #[error(transparent)]
    RandomInit(#[from] RandomBoardInitializerError),
}

// Conversion to a serde-serialisable error for the IPC layer. We render
// each error to a string + a stable kind tag so the frontend can branch
// on the error class without parsing free-form messages.
impl serde::Serialize for SessionError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let kind = match self {
            SessionError::WrongMode { .. } => "wrongMode",
            SessionError::NoBoard => "noBoard",
            SessionError::NoInitialSnapshot => "noInitialSnapshot",
            SessionError::OutOfBounds { .. } => "outOfBounds",
            SessionError::InvalidMaxIterations { .. } => "invalidMaxIterations",
            SessionError::RunCompleted => "runCompleted",
            SessionError::ZeroDimension => "zeroDimension",
            SessionError::Allocation(_) => "allocation",
            SessionError::RandomInit(_) => "randomInit",
        };
        let mut state = s.serialize_struct("SessionError", 2)?;
        state.serialize_field("kind", kind)?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
