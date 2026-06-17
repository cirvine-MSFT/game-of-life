//! Running-phase commands and the background workers behind Play /
//! Jump-to-N.
//!
//! `play` and `jump_to` spawn tokio tasks that hold an `Arc<RunSession>`,
//! call `session.advance_one()` in a tight loop, and emit `board-tick`
//! events. Pause is cooperative: `pause` flips the session's cancel
//! flag, the worker checks it between iterations, exits cleanly, and
//! transitions the session back to `Paused`.
//!
//! The cancel flag is the critic's CRITICAL fix for "Mutex<RunSession>
//! blocks the UI during long advance(N)": the worker's critical
//! section is one generation, not the whole batch, so a Pause click
//! is responsive even at 60 gps with a 10k-iteration Jump in flight.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, State};

use crate::events::{BOARD_TICK, JUMP_PROGRESS, RUN_COMPLETED, SESSION_CHANGED};
use crate::ipc_types::{BoardTick, IpcRunStatus, JumpProgress, Mode, RunCompleted};
use crate::session::{RunSession, SessionError};

/// Lower bound on the play loop's generations-per-second clamp.
pub const MIN_GPS: u16 = 1;
/// Upper bound on the play loop's generations-per-second clamp.
/// Picked so the period (1 / MAX_GPS seconds) stays comfortably above
/// typical IPC + canvas-redraw latency.
pub const MAX_GPS: u16 = 240;
const JUMP_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Clamps a caller-supplied gps value into the supported range. Public
/// so integration tests can assert the bounds.
pub fn clamp_gps(raw: u16) -> u16 {
    raw.clamp(MIN_GPS, MAX_GPS)
}

#[tauri::command]
pub fn start_run(session: State<'_, Arc<RunSession>>) -> Result<(), SessionError> {
    session.start_run()
}

#[tauri::command]
pub fn restart(session: State<'_, Arc<RunSession>>) -> Result<(), SessionError> {
    session.restart()
}

#[tauri::command]
pub fn edit_board(session: State<'_, Arc<RunSession>>) -> Result<(), SessionError> {
    session.edit_board()
}

#[tauri::command]
pub fn extend_max_iterations(
    session: State<'_, Arc<RunSession>>,
    new_total: u64,
) -> Result<(), SessionError> {
    session.extend_max_iterations(new_total)
}

/// Advance exactly one generation. Used by the "Step" toolbar button.
/// Rejects unless the session is in Paused mode so the step cannot
/// race a play/jump worker and corrupt iteration order. The frontend
/// also disables the button outside Paused, but the IPC layer is the
/// last line of defense in case the UI is bypassed.
#[tauri::command]
pub fn step(
    app: AppHandle,
    session: State<'_, Arc<RunSession>>,
) -> Result<(), SessionError> {
    {
        let info = session.info();
        if !matches!(info.mode, Mode::Paused) {
            return Err(SessionError::WrongMode {
                current: info.mode,
                required: "Paused",
            });
        }
    }
    let cloned = session.inner().clone();
    let tick = cloned.advance_one()?;
    let board = cloned.board_payload();
    let _ = app.emit(BOARD_TICK, BoardTick { stats: tick, board });
    emit_completion_if_done(&app, &cloned);
    let _ = app.emit(SESSION_CHANGED, cloned.info());
    Ok(())
}

/// Requests the play / jump worker to stop. Returns immediately; the
/// worker will exit at its next iteration boundary.
#[tauri::command]
pub fn pause(session: State<'_, Arc<RunSession>>) {
    session.request_cancel();
}

/// Starts the play loop at the given generations-per-second rate.
///
/// `gps` is clamped to `MIN_GPS..=MAX_GPS`. The first iteration runs
/// immediately so the user gets visual feedback even at slow rates.
///
/// Uses `begin_playing()` so the mode-check and the transition happen
/// under one lock — closes the TOCTOU race where two concurrent
/// `play` invocations could both observe `Paused` and each spawn a
/// worker.
#[tauri::command]
pub async fn play(
    app: AppHandle,
    session: State<'_, Arc<RunSession>>,
    gps: u16,
) -> Result<(), SessionError> {
    session.begin_playing()?;
    let _ = app.emit(SESSION_CHANGED, session.info());

    let cloned = session.inner().clone();
    let app_for_task = app.clone();
    tokio::spawn(async move {
        run_play_loop(app_for_task, cloned, clamp_gps(gps)).await;
    });
    Ok(())
}

/// Advances toward `target_iteration`. Forward targets `> current` run
/// the simulation; backward targets `< current` restart and replay from
/// generation 0. Progress emits every `JUMP_PROGRESS_INTERVAL`.
///
/// Uses `begin_jumping()` for the atomic Paused -> JumpingTo transition.
#[tauri::command]
pub async fn jump_to(
    app: AppHandle,
    session: State<'_, Arc<RunSession>>,
    target_iteration: u64,
) -> Result<(), SessionError> {
    let current = session.info().iteration;
    if target_iteration < current {
        // `restart` itself stops any (impossible-here) worker and
        // resets to iter 0. Mode is Paused after this so the
        // subsequent `begin_jumping` succeeds.
        session.restart()?;
    }
    session.begin_jumping(target_iteration)?;
    let _ = app.emit(SESSION_CHANGED, session.info());

    let cloned = session.inner().clone();
    let app_for_task = app.clone();
    tokio::spawn(async move {
        run_jump_loop(app_for_task, cloned, target_iteration).await;
    });
    Ok(())
}

async fn run_play_loop(app: AppHandle, session: Arc<RunSession>, gps: u16) {
    let period = Duration::from_micros(1_000_000 / gps as u64);
    let mut next_tick = Instant::now();
    loop {
        if session.cancel_requested() {
            break;
        }
        let tick = match session.advance_one() {
            Ok(tick) => tick,
            Err(_completed) => break,
        };
        let board = session.board_payload();
        let _ = app.emit(BOARD_TICK, BoardTick { stats: tick, board });

        if session.info().completed {
            emit_completion_if_done(&app, &session);
            break;
        }

        next_tick += period;
        let now = Instant::now();
        if next_tick > now {
            tokio::time::sleep(next_tick - now).await;
        } else {
            // Drifted behind schedule (slow advance on huge boards).
            // Reset baseline so we don't burn CPU trying to catch up.
            next_tick = now;
            tokio::task::yield_now().await;
        }
    }

    session.clear_cancel();
    session.set_mode(Mode::Paused);
    let _ = app.emit(SESSION_CHANGED, session.info());
}

async fn run_jump_loop(app: AppHandle, session: Arc<RunSession>, target: u64) {
    let mut last_progress = Instant::now();
    let mut last_emitted: Option<crate::ipc_types::AdvanceTick> = None;
    loop {
        let current = session.info().iteration;
        if current >= target {
            break;
        }
        if session.cancel_requested() {
            break;
        }
        let tick = match session.advance_one() {
            Ok(t) => t,
            Err(_) => break,
        };
        last_emitted = Some(tick);
        // Yield periodically so other tokio tasks (including the IPC
        // request that fired this jump) can progress.
        if last_progress.elapsed() >= JUMP_PROGRESS_INTERVAL {
            let _ = app.emit(
                JUMP_PROGRESS,
                JumpProgress {
                    current: session.info().iteration,
                    target,
                },
            );
            last_progress = Instant::now();
            tokio::task::yield_now().await;
        }
    }

    // One final BOARD_TICK reflecting the *real* last advance — gives
    // the canvas a chance to re-render the post-jump board even when
    // we threw the last in-loop tick away (only progress events were
    // emitted at high jump rates). Skip if we never advanced.
    if let Some(stats) = last_emitted {
        let board = session.board_payload();
        let _ = app.emit(BOARD_TICK, BoardTick { stats, board });
    }

    emit_completion_if_done(&app, &session);

    session.clear_cancel();
    session.set_jump_target(None);
    session.set_mode(Mode::Paused);
    let _ = app.emit(SESSION_CHANGED, session.info());
}

fn emit_completion_if_done(app: &AppHandle, session: &Arc<RunSession>) {
    let info = session.info();
    if !info.completed {
        return;
    }
    if let Some(stats) = session.final_stats() {
        let _ = app.emit(
            RUN_COMPLETED,
            RunCompleted {
                iteration: info.iteration,
                status: info.status.unwrap_or(IpcRunStatus::MaxIterations),
                stats,
            },
        );
    }
}
