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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

use parking_lot::{Mutex, MutexGuard};

use game_of_life::persistence::{
    read_board_snapshot_default, read_run_record_with_warnings, write_board_snapshot,
    BoardSnapshot, BoardSnapshotReadError, BoardSnapshotWriteError, ContentHashMode, FileKind,
    RunRecordReadError,
};
use game_of_life::stats::run_statistics::RunStatus;
use game_of_life::{
    terminal_status_for_outcome, BlinkerBoardInitializer, BoardInitializer, BoardSignature,
    CellState, CycleStatistics, DemoBoardInitializer, FullyAliveInitializer, InMemoryBoard,
    InMemoryBoardCreationError, PatternAnalyzer, PatternBackend, PatternMatchDetails,
    PatternObservation, RandomBoardInitializer, RandomBoardInitializerError, RunStatistics,
    RunStatisticsCollector,
};

use crate::ipc_types::{
    AdvanceTick, BoardPayload, CellEdit, InitialSource, IpcCellState, IpcRunStatistics,
    IpcRunStatus, Mode, PatternName, RunBoardSelection, SessionInfo,
};

/// Upper bound on the alive-count history before in-place decimation.
/// At 60 gps a run can grow this in ~28 minutes; well past that point a
/// straight time series is no longer useful for visual inspection.
const HISTORY_CAP: usize = 100_000;

/// Default memory budget for the board allocation, matching the CLI's
/// `DEFAULT_MAX_BOARD_MEMORY_BYTES` (64 MiB). Reusing the same default
/// keeps semantics consistent between the two front-ends.
pub const DEFAULT_MAX_BOARD_MEMORY_BYTES: usize = 64 * 1024 * 1024;

/// Default run cap for boards loaded outside the normal frontend-created
/// session path. Matches the React store's default new-run cap.
pub const DEFAULT_MAX_ITERATIONS: u64 = 100;

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
    pattern_analyzer: Option<PatternAnalyzer>,
    final_stats: Option<RunStatistics>,
    save_path: Option<PathBuf>,
    saved_snapshot: Option<Vec<u8>>,
    dirty: bool,
    revision: u64,
    worker_generation: u64,
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
            pattern_analyzer: None,
            final_stats: None,
            save_path: None,
            saved_snapshot: None,
            dirty: false,
            revision: 0,
            worker_generation: 0,
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

/// Default play rate used until the first `set_play_rate` arrives.
/// Lives at module level so the play loop's startup branch and the
/// `set_play_rate` IPC can share the same fallback.
pub const DEFAULT_PLAY_RATE_GPS: u16 = 5;

/// Top-level session handle. Cheap to clone via `Arc` from command handlers.
pub struct RunSession {
    inner: Mutex<SessionData>,
    cancel: AtomicBool,
    // Re-read by the play loop on every tick so the user can drag the
    // speed slider live without us pausing and restarting the worker
    // (which produced a visible gap, especially at low rates).
    play_rate_gps: AtomicU16,
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
            play_rate_gps: AtomicU16::new(DEFAULT_PLAY_RATE_GPS),
        }
    }

    fn lock(&self) -> MutexGuard<'_, SessionData> {
        self.inner.lock()
    }

    /// Updates the live play rate. Safe to call at any time — when no
    /// play worker is running it just sits in the atomic until the next
    /// `play()` reads it. The caller is responsible for clamping the
    /// value to the supported range.
    pub fn set_play_rate_gps(&self, gps: u16) {
        self.play_rate_gps.store(gps, Ordering::Relaxed);
    }

    /// Reads the current play rate. Used by the play loop on every tick.
    pub fn play_rate_gps(&self) -> u16 {
        self.play_rate_gps.load(Ordering::Relaxed)
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
            status: data
                .final_stats
                .as_ref()
                .map(|s| IpcRunStatus::from_core(s.status)),
        }
    }

    /// Returns the current cells + dims + iteration as a wire payload.
    pub fn board_payload(&self) -> BoardPayload {
        let mut data = self.lock();
        data.refresh_shadow();
        BoardPayload::from_bytes(data.width, data.height, data.iteration, &data.shadow_buf)
    }

    pub fn board_payload_for_worker(&self, worker_generation: u64) -> Option<BoardPayload> {
        let mut data = self.lock();
        if data.worker_generation != worker_generation {
            return None;
        }
        data.refresh_shadow();
        Some(BoardPayload::from_bytes(
            data.width,
            data.height,
            data.iteration,
            &data.shadow_buf,
        ))
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

    /// Cooperative stop: sets the cancel flag and spin-waits up to ~1s
    /// for any in-flight `play` / `jump_to` worker to acknowledge by
    /// transitioning the session back to `Paused`. Called by mutators
    /// (`restart`, `edit_board`, `create_run`) so they never race the
    /// worker for the lock and then have their state-reset trampled by
    /// the next `advance_one`.
    fn stop_worker_and_wait(&self) {
        let running = matches!(self.lock().mode, Mode::Playing | Mode::JumpingTo);
        if !running {
            return;
        }
        self.cancel.store(true, Ordering::SeqCst);
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            if matches!(self.lock().mode, Mode::Paused | Mode::Setup) {
                return;
            }
        }
        // Worker didn't acknowledge within 1s. Force the mode anyway;
        // the worker will exit on its next iteration and find its
        // mutations already overwritten, which is recoverable.
        self.lock().mode = Mode::Paused;
    }

    /// Atomic Setup -> Setup (no-op) or Paused -> Playing transition
    /// used by the `play` command. Returns the previous mode if it
    /// would have been rejected so the command can error out without
    /// a second lock round-trip. Closes the TOCTOU window where two
    /// concurrent `play` invocations could both see `Paused` and
    /// each spawn a worker.
    pub fn begin_playing(&self) -> Result<u64, SessionError> {
        let mut data = self.lock();
        if !matches!(data.mode, Mode::Paused) {
            return Err(SessionError::WrongMode {
                current: data.mode,
                required: "Paused",
            });
        }
        if data.final_stats.is_some() {
            return Err(SessionError::RunCompleted);
        }
        data.worker_generation = data.worker_generation.wrapping_add(1);
        let worker_generation = data.worker_generation;
        data.mode = Mode::Playing;
        drop(data);
        self.cancel.store(false, Ordering::SeqCst);
        Ok(worker_generation)
    }

    /// Atomic Paused -> JumpingTo transition with the same race-closing
    /// rationale as `begin_playing`. Caller is responsible for the
    /// (optional) backward restart before spawning the worker.
    pub fn begin_jumping(&self, target: u64) -> Result<u64, SessionError> {
        let mut data = self.lock();
        if !matches!(data.mode, Mode::Paused) {
            return Err(SessionError::WrongMode {
                current: data.mode,
                required: "Paused",
            });
        }
        if data.final_stats.is_some() && target > data.iteration {
            return Err(SessionError::RunCompleted);
        }
        data.worker_generation = data.worker_generation.wrapping_add(1);
        let worker_generation = data.worker_generation;
        data.mode = Mode::JumpingTo;
        data.jump_target = Some(target);
        drop(data);
        self.cancel.store(false, Ordering::SeqCst);
        Ok(worker_generation)
    }

    /// Sets up a fresh run in Setup mode. Discards any prior in-flight run.
    /// Memory budget defaults to `DEFAULT_MAX_BOARD_MEMORY_BYTES` when
    /// `max_memory_bytes` is `None`.
    ///
    /// Pre-checks `InMemoryBoard::allocation_bytes` against the budget
    /// before calling `try_new`, mirroring how the CLI decides whether
    /// a board needs streaming. We don't auto-promote yet (issue #10),
    /// so we surface a dedicated `StreamingNotImplemented` error with
    /// a friendly message instead of the lower-level
    /// `MemoryBudgetExceeded`.
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

        // Pre-check before allocating. If the requested board would
        // exceed the budget, fail early with the streaming-aware
        // message; if `allocation_bytes` itself overflows usize the
        // board is genuinely too large for this platform and we
        // surface the underlying allocation error.
        let allocation = InMemoryBoard::allocation_bytes(width as usize, height as usize)?;
        if allocation > budget {
            return Err(SessionError::StreamingNotImplemented {
                width,
                height,
                required_bytes: allocation,
                budget_bytes: budget,
            });
        }

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

        // Stop any in-flight worker BEFORE we mutate session state so
        // the new board doesn't get trampled by a stale advance.
        self.stop_worker_and_wait();

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
        data.pattern_analyzer = None;
        data.final_stats = None;
        data.save_path = None;
        data.saved_snapshot = None;
        data.dirty = false;
        data.revision = data.revision.wrapping_add(1);
        data.worker_generation = data.worker_generation.wrapping_add(1);
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
        let state = if alive {
            CellState::Alive
        } else {
            CellState::Dead
        };
        board.set(x as usize, y as usize, state);
        data.dirty = true;
        data.revision = data.revision.wrapping_add(1);
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
            data.revision = data.revision.wrapping_add(1);
        }
        Ok(())
    }

    pub fn apply_pattern(&self, pattern: PatternName) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        apply_pattern_to(board, pattern);
        data.dirty = true;
        data.revision = data.revision.wrapping_add(1);
        Ok(())
    }

    pub fn randomize(&self, seed: u64, alive_cells_per_thousand: u16) -> Result<(), SessionError> {
        let mut data = self.lock();
        require_setup(&data)?;
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        let init =
            RandomBoardInitializer::with_alive_cells_per_thousand(seed, alive_cells_per_thousand)?;
        init.initialize(board)
            .expect("InMemoryBoard editor is infallible");
        data.dirty = true;
        data.revision = data.revision.wrapping_add(1);
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
        data.revision = data.revision.wrapping_add(1);
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
        let initial_signature =
            BoardSignature::from_view(board).expect("InMemoryBoard signatures are infallible");
        let initial_alive_count = initial_signature.alive_count();
        let mut analyzer = PatternAnalyzer::in_memory_cycle_detection();
        if initial_alive_count != 0 {
            analyzer.observe(&PatternObservation::new(
                0,
                PatternBackend::InMemory,
                None,
                Some(&initial_signature),
            ));
        }
        data.initial_snapshot = Some(initial_snapshot);
        data.iteration = 0;
        data.alive_history = vec![initial_alive_count];
        data.stats = Some(RunStatisticsCollector::starting_from(initial_alive_count));
        data.pattern_analyzer = Some(analyzer);
        data.final_stats = None;
        data.mode = Mode::Paused;
        data.jump_target = None;
        self.cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Restores the board to its initial snapshot, resets iteration and
    /// stats history. Caller must be in a Running mode. If a play /
    /// jump worker is in flight it is cancelled and awaited before the
    /// rewind happens, so the worker can't trample the restored state
    /// with a stale `advance_one`.
    pub fn restart(&self) -> Result<(), SessionError> {
        if !matches!(
            self.lock().mode,
            Mode::Paused | Mode::Playing | Mode::JumpingTo
        ) {
            return Err(SessionError::WrongMode {
                current: self.lock().mode,
                required: "Running",
            });
        }
        self.stop_worker_and_wait();
        let mut data = self.lock();
        let snapshot = data
            .initial_snapshot
            .clone()
            .ok_or(SessionError::NoInitialSnapshot)?;
        let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
        write_cells(board, &snapshot);
        let initial_signature =
            BoardSignature::from_view(board).expect("InMemoryBoard signatures are infallible");
        let initial_alive_count = initial_signature.alive_count();
        let mut analyzer = PatternAnalyzer::in_memory_cycle_detection();
        if initial_alive_count != 0 {
            analyzer.observe(&PatternObservation::new(
                0,
                PatternBackend::InMemory,
                None,
                Some(&initial_signature),
            ));
        }
        data.iteration = 0;
        data.alive_history = vec![initial_alive_count];
        data.stats = Some(RunStatisticsCollector::starting_from(initial_alive_count));
        data.pattern_analyzer = Some(analyzer);
        data.final_stats = None;
        data.mode = Mode::Paused;
        data.dirty = data
            .saved_snapshot
            .as_ref()
            .map(|saved| saved != &snapshot)
            .unwrap_or(true);
        data.revision = data.revision.wrapping_add(1);
        data.worker_generation = data.worker_generation.wrapping_add(1);
        data.jump_target = None;
        Ok(())
    }

    /// Exits Running mode back into Setup, dropping the run record but
    /// keeping the current board (so the user can keep painting). Any
    /// in-flight worker is stopped and awaited first.
    pub fn edit_board(&self) -> Result<(), SessionError> {
        if matches!(self.lock().mode, Mode::Setup) {
            return Ok(());
        }
        self.stop_worker_and_wait();
        let mut data = self.lock();
        data.mode = Mode::Setup;
        data.initial_snapshot = None;
        data.iteration = 0;
        data.alive_history = Vec::new();
        data.stats = None;
        data.pattern_analyzer = None;
        data.final_stats = None;
        data.worker_generation = data.worker_generation.wrapping_add(1);
        data.jump_target = None;
        Ok(())
    }

    /// Raises `max_iterations`. Useful for Run-to-N where the user wants
    /// the simulation to keep going past its original cap. If the run
    /// had already terminated via `MaxIterations`, the stats collector
    /// is rehydrated from the last alive count so subsequent advances
    /// continue accumulating into the same series instead of looping
    /// forever past the cap (the play worker breaks on
    /// `info().completed`, which would otherwise stay false because
    /// `advance_one` couldn't finalise without a live collector).
    pub fn extend_max_iterations(&self, new_total: u64) -> Result<(), SessionError> {
        let mut data = self.lock();
        if new_total < data.iteration {
            return Err(SessionError::InvalidMaxIterations { new_total });
        }
        data.max_iterations = new_total;
        let was_at_max = data
            .final_stats
            .as_ref()
            .map(|s| matches!(s.status, RunStatus::MaxIterations))
            .unwrap_or(false);
        if was_at_max && new_total > data.iteration {
            let restored_stats = if data.stats.is_none() {
                data.final_stats
                    .as_ref()
                    .map(RunStatisticsCollector::from_statistics)
            } else {
                None
            };
            data.final_stats = None;
            if let Some(stats) = restored_stats {
                data.stats = Some(stats);
            }
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

    pub fn finish_worker(&self, worker_generation: u64) {
        let mut data = self.lock();
        if data.worker_generation != worker_generation {
            return;
        }
        data.jump_target = None;
        if !matches!(data.mode, Mode::Setup) {
            data.mode = Mode::Paused;
        }
        self.cancel.store(false, Ordering::SeqCst);
    }

    pub fn worker_should_stop(&self, worker_generation: u64) -> bool {
        let data = self.lock();
        data.worker_generation != worker_generation || self.cancel.load(Ordering::SeqCst)
    }

    pub fn worker_is_current(&self, worker_generation: u64) -> bool {
        self.lock().worker_generation == worker_generation
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

    /// Advances by exactly one generation. A no-op advance is treated as a
    /// stability confirmation for the previous board, so it finalizes the run
    /// without incrementing the visible iteration counter.
    pub fn advance_one(&self) -> Result<AdvanceTick, SessionError> {
        self.advance_one_inner(None)
    }

    pub fn advance_one_for_worker(
        &self,
        worker_generation: u64,
    ) -> Result<AdvanceTick, SessionError> {
        self.advance_one_inner(Some(worker_generation))
    }

    fn advance_one_inner(
        &self,
        worker_generation: Option<u64>,
    ) -> Result<AdvanceTick, SessionError> {
        let mut data = self.lock();
        if let Some(worker_generation) = worker_generation {
            if data.worker_generation != worker_generation {
                return Err(SessionError::WorkerStopped);
            }
        }
        if data.final_stats.is_some() {
            return Err(SessionError::RunCompleted);
        }
        let summary = {
            let board = data.board.as_mut().ok_or(SessionError::NoBoard)?;
            board.advance_generation_with_signature()
        };
        let outcome = summary.outcome;
        if !outcome.is_stable() {
            data.dirty = true;
            data.revision = data.revision.wrapping_add(1);
        }

        let total_cells = data.width as u64 * data.height as u64;
        let terminal_status = terminal_status_for_outcome(outcome);
        if outcome.is_stable() {
            let tick = AdvanceTick::from_outcome(data.iteration, total_cells, outcome);
            if let Some(stats) = data.stats.take() {
                let status = terminal_status.unwrap_or(RunStatus::Stable);
                data.final_stats = Some(stats.finalize(status));
            }
            data.mode = Mode::Paused;
            self.cancel.store(false, Ordering::SeqCst);
            return Ok(tick);
        }

        data.iteration += 1;

        if let Some(stats) = data.stats.as_mut() {
            stats.record(outcome);
        }
        data.push_history(outcome.alive_count);

        let tick = AdvanceTick::from_outcome(data.iteration, total_cells, outcome);
        let cycle_stats = if terminal_status.is_none() {
            let generation = data.iteration;
            match (data.pattern_analyzer.as_mut(), summary.signature.as_ref()) {
                (Some(analyzer), Some(signature)) => {
                    let observation = PatternObservation::new(
                        generation,
                        PatternBackend::InMemory,
                        Some(outcome),
                        Some(signature),
                    );
                    analyzer.observe(&observation).map(|pattern_match| {
                        let PatternMatchDetails::Cycle(cycle) = pattern_match.details;
                        CycleStatistics {
                            start_generation: cycle.start_generation,
                            detected_generation: cycle.detected_generation,
                            period: cycle.period,
                        }
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        // Terminal-state detection: extinction/stability takes priority over
        // the iteration ceiling so the more specific status wins on edge
        // cases where both happen on the same generation.
        let hit_cap = data.iteration >= data.max_iterations;
        if terminal_status.is_some() || cycle_stats.is_some() || hit_cap {
            let status = terminal_status.unwrap_or_else(|| {
                if cycle_stats.is_some() {
                    RunStatus::Cyclic
                } else {
                    RunStatus::MaxIterations
                }
            });
            if let Some(stats) = data.stats.take() {
                data.final_stats = Some(stats.finalize_with_cycle(status, cycle_stats));
            }
            data.mode = Mode::Paused;
            self.cancel.store(false, Ordering::SeqCst);
        }

        Ok(tick)
    }

    /// Saves the *current* board (whatever iteration we're on) as a
    /// standalone `GOL-BOARD-SNAPSHOT v1` file at `path`. Mirrors the
    /// CLI's `--save-board` escape hatch so users can persist a board
    /// of interest without waiting for the run to terminate.
    ///
    /// Wraps `write_board_snapshot`, which refuses to overwrite — the
    /// frontend is expected to handle overwrite confirmation by
    /// removing the file first if the user OK's it.
    pub fn save_board_snapshot(&self, path: &Path) -> Result<(), SessionError> {
        let board = {
            let data = self.lock();
            (
                data.board.clone().ok_or(SessionError::NoBoard)?,
                data.revision,
            )
        };
        let saved_snapshot = collect_cells(&board.0);
        let snapshot = BoardSnapshot::for_board(board.0);
        write_board_snapshot(path, &snapshot).map_err(|e| match e {
            BoardSnapshotWriteError::OutputExists { path } => SessionError::SaveBoardSnapshot(
                format!("Refusing to overwrite existing file '{}'", path.display()),
            ),
            BoardSnapshotWriteError::Io(io) => SessionError::SaveBoardSnapshot(io.to_string()),
        })?;
        let mut data = self.lock();
        data.save_path = Some(path.to_path_buf());
        data.saved_snapshot = Some(saved_snapshot);
        if data.revision == board.1 {
            data.dirty = false;
        }
        Ok(())
    }

    /// Loads a standalone `GOL-BOARD-SNAPSHOT v1` file into Setup mode.
    /// Invalid files leave the current session untouched.
    pub fn load_board_snapshot(&self, path: &Path) -> Result<(), SessionError> {
        let snapshot = match read_board_snapshot_default(path, DEFAULT_MAX_BOARD_MEMORY_BYTES) {
            Ok(snapshot) => snapshot,
            Err(BoardSnapshotReadError::UnexpectedFileKind {
                actual: FileKind::RunRecord,
                ..
            }) => {
                return self.load_run_board(path, RunBoardSelection::Initial);
            }
            Err(e) => {
                return Err(SessionError::LoadBoardSnapshot(
                    load_snapshot_error_message(e),
                ));
            }
        };
        self.load_board_into_setup(snapshot.board, Some(path.to_path_buf()), None)
    }

    pub fn load_run_board(
        &self,
        path: &Path,
        selection: RunBoardSelection,
    ) -> Result<(), SessionError> {
        let loaded = read_run_record_with_warnings(
            path,
            DEFAULT_MAX_BOARD_MEMORY_BYTES,
            game_of_life::persistence::DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .map_err(|e| SessionError::LoadRunRecord(load_run_error_message(e)))?;
        let max_iterations =
            u64::try_from(loaded.record.config.max_iterations).unwrap_or(DEFAULT_MAX_ITERATIONS);
        let board = match selection {
            RunBoardSelection::Initial => loaded.record.initial_board,
            RunBoardSelection::Final => loaded.record.final_board,
        };
        self.load_board_into_setup(board, None, Some(max_iterations))
    }

    fn load_board_into_setup(
        &self,
        board: InMemoryBoard,
        save_path: Option<PathBuf>,
        max_iterations: Option<u64>,
    ) -> Result<(), SessionError> {
        let saved_snapshot = collect_cells(&board);
        let width = u32::try_from(board.width()).map_err(|_| {
            SessionError::LoadBoardSnapshot(format!(
                "Loaded board width {} exceeds desktop limits.",
                board.width()
            ))
        })?;
        let height = u32::try_from(board.height()).map_err(|_| {
            SessionError::LoadBoardSnapshot(format!(
                "Loaded board height {} exceeds desktop limits.",
                board.height()
            ))
        })?;

        self.stop_worker_and_wait();

        let mut data = self.lock();
        let max_iterations = max_iterations.unwrap_or_else(|| {
            if data.max_iterations == 0 {
                DEFAULT_MAX_ITERATIONS
            } else {
                data.max_iterations
            }
        });
        data.mode = Mode::Setup;
        data.width = width;
        data.height = height;
        data.board = Some(board);
        data.initial_snapshot = None;
        data.iteration = 0;
        data.max_iterations = max_iterations;
        data.alive_history = Vec::new();
        data.stats = None;
        data.pattern_analyzer = None;
        data.final_stats = None;
        data.save_path = save_path;
        data.saved_snapshot = Some(saved_snapshot);
        data.dirty = false;
        data.revision = data.revision.wrapping_add(1);
        data.worker_generation = data.worker_generation.wrapping_add(1);
        data.jump_target = None;
        data.shadow_buf = Vec::new();
        Ok(())
    }
}

fn load_snapshot_error_message(error: BoardSnapshotReadError) -> String {
    match error {
        BoardSnapshotReadError::Io(_) => {
            "Could not read the selected board snapshot. Confirm the file still exists and is readable."
                .to_string()
        }
        BoardSnapshotReadError::UnexpectedFileKind {
            expected, actual, ..
        } => format!("Selected file is a {actual}, but expected a {expected}."),
        BoardSnapshotReadError::LoadedBoardSize(e) => e.to_string(),
        BoardSnapshotReadError::FileTooLarge {
            actual_bytes,
            limit_bytes,
            ..
        } => format!(
            "Selected file is {actual_bytes} bytes, which exceeds the {limit_bytes}-byte input file limit."
        ),
        BoardSnapshotReadError::MalformedSizeHeader { .. } => {
            "Selected board snapshot has a malformed size header. Expected WIDTHxHEIGHT, for example 10x10."
                .to_string()
        }
        BoardSnapshotReadError::Magic(_)
        | BoardSnapshotReadError::InvalidTimestamp(_)
        | BoardSnapshotReadError::Parse(_) => {
            "Selected file is not a valid Game of Life board snapshot. Expected a GOL-BOARD-SNAPSHOT v1 .gol file with a #/. board grid."
                .to_string()
        }
    }
}

fn load_run_error_message(error: RunRecordReadError) -> String {
    match error {
        RunRecordReadError::Io(_) => {
            "Could not read the selected run record. Confirm the file still exists and is readable."
                .to_string()
        }
        RunRecordReadError::UnexpectedFileKind { actual, .. } => {
            format!("Selected file is a {actual}, but expected a run record.")
        }
        RunRecordReadError::BoardBlockTooLarge { source, .. } => source.to_string(),
        RunRecordReadError::FileTooLarge {
            actual_bytes,
            limit_bytes,
            ..
        } => format!(
            "Selected file is {actual_bytes} bytes, which exceeds the {limit_bytes}-byte input file limit."
        ),
        RunRecordReadError::Corrupted { .. } | RunRecordReadError::MissingContentHash { .. } => {
            "Selected run record failed integrity validation. Recreate the run record, or extract a board snapshot from it with the CLI if you intentionally edited it."
                .to_string()
        }
        RunRecordReadError::MalformedSizeHeader { .. } => {
            "Selected run record has a malformed board size header. Expected WIDTHxHEIGHT, for example 10x10."
                .to_string()
        }
        RunRecordReadError::Magic(_)
        | RunRecordReadError::InvalidTimestamp(_)
        | RunRecordReadError::Parse(_)
        | RunRecordReadError::UnrecognizedStatus { .. }
        | RunRecordReadError::MalformedRunId { .. }
        | RunRecordReadError::MalformedField { .. }
        | RunRecordReadError::MissingField { .. } => {
            "Selected file is not a valid Game of Life run record. Expected a GOL-RUN-RECORD v1 or v2 .gol file."
                .to_string()
        }
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
            let state = if alive {
                CellState::Alive
            } else {
                CellState::Dead
            };
            board.set(x, y, state);
        }
    }
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
    #[error("worker stopped")]
    WorkerStopped,
    #[error("board width and height must be greater than zero")]
    ZeroDimension,
    #[error(
        "Board {width}x{height} needs {required_bytes} bytes but the desktop budget is {budget_bytes}. \
        Streaming-mode boards (which the CLI supports via --working-dir) are not yet wired into the \
        desktop visualizer — see issue #10. Pick a smaller board or raise the budget."
    )]
    StreamingNotImplemented {
        width: u32,
        height: u32,
        required_bytes: usize,
        budget_bytes: usize,
    },
    #[error("save board snapshot failed: {0}")]
    SaveBoardSnapshot(String),
    #[error("load board snapshot failed: {0}")]
    LoadBoardSnapshot(String),
    #[error("load run record failed: {0}")]
    LoadRunRecord(String),
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
            SessionError::WorkerStopped => "workerStopped",
            SessionError::ZeroDimension => "zeroDimension",
            SessionError::StreamingNotImplemented { .. } => "streamingNotImplemented",
            SessionError::SaveBoardSnapshot(_) => "saveBoardSnapshot",
            SessionError::LoadBoardSnapshot(_) => "loadBoardSnapshot",
            SessionError::LoadRunRecord(_) => "loadRunRecord",
            SessionError::Allocation(_) => "allocation",
            SessionError::RandomInit(_) => "randomInit",
        };
        let mut state = s.serialize_struct("SessionError", 2)?;
        state.serialize_field("kind", kind)?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
