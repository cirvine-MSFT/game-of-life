// Typed wrappers around `invoke()` for every Tauri command in
// `desktop/src/commands/`. The function names match the Rust command
// names so the IPC contract is greppable from either side.

import { invoke } from "@tauri-apps/api/core";

import type {
  BoardPayload,
  CellEdit,
  InitialSource,
  IpcRunSeries,
  IpcRunStatistics,
  PatternName,
  RunBoardSelection,
  SessionInfo,
} from "./types";

// --- session_commands ---

export const getSession = (): Promise<SessionInfo> => invoke("get_session");

export const getBoard = (): Promise<BoardPayload> => invoke("get_board");

export const getAliveHistory = (): Promise<number[]> =>
  invoke("get_alive_history");

export const getFinalStats = (): Promise<IpcRunStatistics | null> =>
  invoke("get_final_stats");

export const readRunSeries = (path: string): Promise<IpcRunSeries> =>
  invoke("read_run_series", { path });

export const defaultSaveDir = (): Promise<string> => invoke("default_save_dir");

export const saveBoardSnapshot = (
  path: string,
  overwrite: boolean,
): Promise<string> => invoke("save_board_snapshot", { path, overwrite });

export const loadBoardSnapshot = (path: string): Promise<string> =>
  invoke("load_board_snapshot", { path });

export const loadRunBoard = (
  path: string,
  selection: RunBoardSelection,
): Promise<string> => invoke("load_run_board", { path, selection });

// --- setup_commands ---

export interface CreateRunArgs {
  width: number;
  height: number;
  source: InitialSource;
  maxIterations: number;
  maxMemoryBytes?: number;
}

export const createRun = (args: CreateRunArgs): Promise<void> =>
  invoke("create_run", args as unknown as Record<string, unknown>);

export const setCell = (x: number, y: number, alive: boolean): Promise<void> =>
  invoke("set_cell", { x, y, alive });

export const paintCells = (edits: CellEdit[]): Promise<void> =>
  invoke("paint_cells", { edits });

export const applyPattern = (pattern: PatternName): Promise<void> =>
  invoke("apply_pattern", { pattern });

export const randomize = (
  seed: number,
  aliveCellsPerThousand: number,
): Promise<void> => invoke("randomize", { seed, aliveCellsPerThousand });

export const clearBoard = (): Promise<void> => invoke("clear_board");

// --- run_commands ---

export const startRun = (): Promise<void> => invoke("start_run");
export const restart = (): Promise<void> => invoke("restart");
export const editBoard = (): Promise<void> => invoke("edit_board");
export const step = (): Promise<void> => invoke("step");
export const pause = (): Promise<void> => invoke("pause");
export const play = (gps: number): Promise<void> => invoke("play", { gps });
export const jumpTo = (targetIteration: number): Promise<void> =>
  invoke("jump_to", { targetIteration });
export const extendMaxIterations = (newTotal: number): Promise<void> =>
  invoke("extend_max_iterations", { newTotal });
