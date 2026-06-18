// TypeScript mirrors of the Rust wire types in `desktop/src/ipc_types.rs`.
//
// Keeping these in lock-step with the Rust side is part of the
// `ipc-types` todo's contract. When the Rust types change, this file
// changes in the same PR.

export type Mode = "setup" | "paused" | "playing" | "jumpingTo";

export type IpcRunStatus = "maxIterations" | "extinct" | "stable";

export type PatternName = "demo" | "blinker" | "fullyAlive";

export type InitialSource =
  | { kind: "empty" }
  | { kind: "pattern"; value: PatternName }
  | {
      kind: "random";
      value: { seed: number; aliveCellsPerThousand: number };
    };

export interface BoardPayload {
  width: number;
  height: number;
  iteration: number;
  cellsBase64: string;
}

export interface AdvanceTick {
  iteration: number;
  alive: number;
  dead: number;
  births: number;
  deaths: number;
}

export interface BoardTick extends AdvanceTick {
  board: BoardPayload;
}

export interface JumpProgress {
  current: number;
  target: number;
}

export interface IpcRunStatistics {
  initialAliveCount: number;
  finalAliveCount: number;
  peakAliveCount: number;
  peakAliveGeneration: number;
  minAliveCount: number;
  minAliveGeneration: number;
  totalBirths: number;
  totalDeaths: number;
  iterationsRun: number;
  status: IpcRunStatus;
}

export interface RunCompleted {
  iteration: number;
  status: IpcRunStatus;
  stats: IpcRunStatistics;
}

export interface SessionInfo {
  mode: Mode;
  iteration: number;
  width: number;
  height: number;
  maxIterations: number;
  savePath: string | null;
  dirty: boolean;
  completed: boolean;
  jumpTarget: number | null;
  status: IpcRunStatus | null;
}

export interface CellEdit {
  x: number;
  y: number;
  alive: boolean;
}

// The shape of the structured error the Rust SessionError serialises as.
export interface SessionErrorPayload {
  kind:
    | "wrongMode"
    | "noBoard"
    | "noInitialSnapshot"
    | "outOfBounds"
    | "invalidMaxIterations"
    | "runCompleted"
    | "zeroDimension"
    | "allocation"
    | "randomInit";
  message: string;
}

// Decoded view of a BoardPayload: cells as a typed array with width/height
// for direct indexing.
export interface DecodedBoard {
  width: number;
  height: number;
  iteration: number;
  cells: Uint8Array;
}

export const decodeBoard = (payload: BoardPayload): DecodedBoard => {
  const binary = atob(payload.cellsBase64);
  const cells = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    cells[i] = binary.charCodeAt(i);
  }
  return {
    width: payload.width,
    height: payload.height,
    iteration: payload.iteration,
    cells,
  };
};
