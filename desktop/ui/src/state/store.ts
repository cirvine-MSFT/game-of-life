// Application-wide Zustand store.
//
// Holds session metadata (mode, iteration, dims, save state), the
// currently-rendered board snapshot, alive-count history for the chart,
// the last per-generation stats tick, and the user's theme preference.
// Every action that mutates Rust-side state goes through an IPC command
// here so the store and the session never drift.
//
// Event subscription lives in `connect()`. Components import the hook
// and call `connect()` once at app mount inside a `useEffect`. The store
// guards against double subscription via the `connected` flag so React
// strict-mode double-mounting is harmless.

import { create } from "zustand";
import type { UnlistenFn } from "@tauri-apps/api/event";

import {
  applyPattern,
  clearBoard,
  createRun,
  decodeBoard,
  editBoard,
  extendMaxIterations,
  getAliveHistory,
  getBoard,
  getFinalStats,
  getSession,
  jumpTo,
  onBoardTick,
  onJumpProgress,
  onRunCompleted,
  onSessionChanged,
  paintCells,
  pause,
  play,
  randomize,
  restart,
  setCell,
  startRun,
  step,
  type CreateRunArgs,
  type DecodedBoard,
  type IpcRunStatistics,
  type JumpProgress,
  type PatternName,
  type SessionInfo,
} from "../ipc";

export type ThemeChoice = "light" | "dark" | "highContrast" | "system";

interface TickSummary {
  iteration: number;
  alive: number;
  dead: number;
  births: number;
  deaths: number;
}

interface AppState {
  // Reactive state
  session: SessionInfo | null;
  board: DecodedBoard | null;
  history: number[];
  latestTick: TickSummary | null;
  jumpProgress: JumpProgress | null;
  finalStats: IpcRunStatistics | null;
  theme: ThemeChoice;
  connected: boolean;
  initError: string | null;

  // Lifecycle
  connect: () => Promise<void>;
  refreshSession: () => Promise<void>;
  refreshBoard: () => Promise<void>;
  refreshHistory: () => Promise<void>;
  refreshFinalStats: () => Promise<void>;

  // Setup actions
  newRun: (args: CreateRunArgs) => Promise<void>;
  setCell: (x: number, y: number, alive: boolean) => Promise<void>;
  paintCells: (edits: { x: number; y: number; alive: boolean }[]) => Promise<void>;
  applyPattern: (pattern: PatternName) => Promise<void>;
  randomize: (seed: number, aliveCellsPerThousand: number) => Promise<void>;
  clearBoard: () => Promise<void>;

  // Run actions
  startRun: () => Promise<void>;
  play: (gps: number) => Promise<void>;
  pause: () => Promise<void>;
  step: () => Promise<void>;
  restart: () => Promise<void>;
  jumpTo: (target: number) => Promise<void>;
  extendMaxIterations: (newTotal: number) => Promise<void>;
  editBoard: () => Promise<void>;

  // Settings
  setTheme: (theme: ThemeChoice) => void;

  // Tear-down
  disconnect: () => void;
}

let unlistens: UnlistenFn[] = [];

const DEFAULT_NEW_RUN: CreateRunArgs = {
  width: 20,
  height: 20,
  source: { kind: "pattern", value: "demo" },
  maxIterations: 100,
};

export const useStore = create<AppState>((set, get) => ({
  session: null,
  board: null,
  history: [],
  latestTick: null,
  jumpProgress: null,
  finalStats: null,
  theme: "light",
  connected: false,
  initError: null,

  connect: async () => {
    if (get().connected) {
      return;
    }
    // Don't latch `connected: true` before the subscription chain — if
    // anything throws, the catch clause needs to reset the flag so the
    // user's Retry button can re-enter this method. Mark a separate
    // "connecting" state until everything is in place.
    set({ initError: null });
    const localUnlistens: UnlistenFn[] = [];
    try {
      const session = await getSession();
      set({ session });

      // Auto-create a default run if none exists so the canvas always has
      // something to render. The user can replace it via File -> New Run.
      if (session.width === 0) {
        await get().newRun(DEFAULT_NEW_RUN);
      } else {
        await get().refreshBoard();
        await get().refreshHistory();
      }

      // Register listeners sequentially so a mid-list failure can still
      // tear down whatever already registered. Promise.all() would leak
      // the earlier-resolved unlisten functions on a later rejection.
      localUnlistens.push(
        await onBoardTick((tick) => {
          set({
            board: decodeBoard(tick.board),
            latestTick: {
              iteration: tick.iteration,
              alive: tick.alive,
              dead: tick.dead,
              births: tick.births,
              deaths: tick.deaths,
            },
            history: [...get().history, tick.alive],
          });
        }),
      );
      localUnlistens.push(
        await onJumpProgress((progress) => {
          set({ jumpProgress: progress });
        }),
      );
      localUnlistens.push(
        await onRunCompleted((completion) => {
          set({
            finalStats: completion.stats,
            jumpProgress: null,
          });
        }),
      );
      localUnlistens.push(
        await onSessionChanged((info) => {
          set({ session: info });
        }),
      );
      unlistens = localUnlistens;
      set({ connected: true });
    } catch (error) {
      // Clean up any listeners that did register before the failure.
      for (const fn of localUnlistens) {
        try {
          fn();
        } catch {
          // Tear-down errors here are non-actionable; swallow.
        }
      }
      set({
        connected: false,
        initError: error instanceof Error ? error.message : String(error),
      });
    }
  },

  refreshSession: async () => {
    set({ session: await getSession() });
  },

  refreshBoard: async () => {
    const payload = await getBoard();
    set({ board: decodeBoard(payload) });
  },

  refreshHistory: async () => {
    set({ history: await getAliveHistory() });
  },

  refreshFinalStats: async () => {
    set({ finalStats: await getFinalStats() });
  },

  newRun: async (args) => {
    await createRun(args);
    await get().refreshSession();
    await get().refreshBoard();
    set({ history: [], latestTick: null, finalStats: null, jumpProgress: null });
  },

  setCell: async (x, y, alive) => {
    await setCell(x, y, alive);
    await get().refreshBoard();
    await get().refreshSession();
  },

  paintCells: async (edits) => {
    await paintCells(edits);
    await get().refreshBoard();
    await get().refreshSession();
  },

  applyPattern: async (pattern) => {
    await applyPattern(pattern);
    await get().refreshBoard();
    await get().refreshSession();
  },

  randomize: async (seed, aliveCellsPerThousand) => {
    await randomize(seed, aliveCellsPerThousand);
    await get().refreshBoard();
    await get().refreshSession();
  },

  clearBoard: async () => {
    await clearBoard();
    await get().refreshBoard();
    await get().refreshSession();
  },

  startRun: async () => {
    await startRun();
    await get().refreshSession();
    await get().refreshHistory();
    set({ finalStats: null });
  },

  play: async (gps) => {
    await play(gps);
    await get().refreshSession();
  },

  pause: async () => {
    await pause();
    await get().refreshSession();
  },

  step: async () => {
    await step();
  },

  restart: async () => {
    await restart();
    await get().refreshBoard();
    await get().refreshHistory();
    await get().refreshSession();
    set({ finalStats: null, latestTick: null });
  },

  jumpTo: async (target) => {
    await jumpTo(target);
    await get().refreshSession();
  },

  extendMaxIterations: async (newTotal) => {
    await extendMaxIterations(newTotal);
    await get().refreshSession();
  },

  editBoard: async () => {
    await editBoard();
    await get().refreshSession();
    set({ history: [], latestTick: null, finalStats: null, jumpProgress: null });
  },

  setTheme: (theme) => {
    set({ theme });
  },

  disconnect: () => {
    for (const fn of unlistens) {
      try {
        fn();
      } catch {
        // Tear-down errors are non-actionable; swallow.
      }
    }
    unlistens = [];
    set({ connected: false });
  },
}));
