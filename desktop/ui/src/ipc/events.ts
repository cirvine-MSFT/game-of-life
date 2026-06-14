// Event-name constants and typed `listen()` helpers that mirror the Rust
// `desktop/src/events.rs` module.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { BoardTick, JumpProgress, RunCompleted, SessionInfo } from "./types";

export const EVENT_BOARD_TICK = "gol://board-tick";
export const EVENT_JUMP_PROGRESS = "gol://jump-progress";
export const EVENT_RUN_COMPLETED = "gol://run-completed";
export const EVENT_SESSION_CHANGED = "gol://session-changed";
export const EVENT_ERROR = "gol://error";

const subscribe = <T,>(
  name: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> =>
  listen<T>(name, (event) => {
    handler(event.payload);
  });

export const onBoardTick = (handler: (tick: BoardTick) => void) =>
  subscribe<BoardTick>(EVENT_BOARD_TICK, handler);

export const onJumpProgress = (handler: (progress: JumpProgress) => void) =>
  subscribe<JumpProgress>(EVENT_JUMP_PROGRESS, handler);

export const onRunCompleted = (handler: (completion: RunCompleted) => void) =>
  subscribe<RunCompleted>(EVENT_RUN_COMPLETED, handler);

export const onSessionChanged = (handler: (info: SessionInfo) => void) =>
  subscribe<SessionInfo>(EVENT_SESSION_CHANGED, handler);
