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
  defaultSaveDir,
  editBoard,
  extendMaxIterations,
  getAliveHistory,
  getBoard,
  getFinalStats,
  getSession,
  jumpTo,
  loadBoardSnapshot,
  loadRunBoard,
  onBoardTick,
  onJumpProgress,
  onRunCompleted,
  onSessionChanged,
  paintCells,
  pause,
  play,
  setPlayRate,
  randomize,
  readRunSeries,
  restart,
  saveBoardSnapshot,
  setCell,
  startRun,
  step,
  type CreateRunArgs,
  type DecodedBoard,
  type IpcIterationSeries,
  type IpcRunStatistics,
  type JumpProgress,
  type PatternName,
  type RunBoardSelection,
  type SessionInfo,
} from "../ipc";

export type ThemeChoice = "light" | "dark" | "highContrast" | "system";

export type ActiveView =
  | "edit"
  | "run"
  | "aggregate"
  | "settings"
  | "telemetry";

export type AggregateRowStatus =
  | "loading"
  | "ready"
  | "summaryOnly"
  | "error";

export interface AggregateRow {
  path: string;
  filename: string;
  status: AggregateRowStatus;
  colorIndex: number;
  visible: boolean;
  series?: IpcIterationSeries;
  summary?: IpcRunStatistics;
  error?: string;
}

const ACTIVE_VIEW_STORAGE_KEY = "gol.activeView";
const VALID_PERSISTED_VIEWS: readonly ActiveView[] = [
  "edit",
  "run",
  "aggregate",
  "settings",
];

// "telemetry" is part of the type so a future pane can be wired up by
// flipping the disabled flag on its nav-rail tab, but it doesn't render
// today. Coerce any persisted "telemetry" (or any unknown value) back to
// "edit" so the user can never boot into an unreachable view.
//
// Exported so tests can exercise the coercion without needing to spin up
// a fresh module instance via `vi.resetModules` + dynamic import.
export const loadPersistedActiveView = (): ActiveView => {
  try {
    const raw =
      typeof localStorage !== "undefined"
        ? localStorage.getItem(ACTIVE_VIEW_STORAGE_KEY)
        : null;
    if (raw && (VALID_PERSISTED_VIEWS as readonly string[]).includes(raw)) {
      return raw as ActiveView;
    }
  } catch {
    // localStorage can throw in restricted contexts; fall through to default.
  }
  return "edit";
};

const persistActiveView = (view: ActiveView): void => {
  try {
    if (typeof localStorage === "undefined") return;
    if (view === "telemetry") {
      // Don't persist a value we'd just coerce away on load.
      localStorage.removeItem(ACTIVE_VIEW_STORAGE_KEY);
      return;
    }
    localStorage.setItem(ACTIVE_VIEW_STORAGE_KEY, view);
  } catch {
    // Persistence is best-effort; swallow.
  }
};

const ANIMATE_TRANSITIONS_STORAGE_KEY = "gol.animateTransitions";

// Default to true so the animation that ships in the box is what new
// users see; persistence lets people who find it distracting turn it
// off and have that stick across sessions.
export const loadPersistedAnimateTransitions = (): boolean => {
  try {
    const raw =
      typeof localStorage !== "undefined"
        ? localStorage.getItem(ANIMATE_TRANSITIONS_STORAGE_KEY)
        : null;
    if (raw === "false") return false;
    if (raw === "true") return true;
  } catch {
    // localStorage can throw in restricted contexts; fall through.
  }
  return true;
};

const persistAnimateTransitions = (value: boolean): void => {
  try {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(ANIMATE_TRANSITIONS_STORAGE_KEY, String(value));
  } catch {
    // Persistence is best-effort; swallow.
  }
};

interface TickSummary {
  iteration: number;
  alive: number;
  dead: number;
  births: number;
  deaths: number;
}

export interface LoadedReference {
  path: string;
  filename: string;
  summaryOnly: boolean;
}

interface AppState {
  // Reactive state
  session: SessionInfo | null;
  board: DecodedBoard | null;
  history: number[];
  latestTick: TickSummary | null;
  jumpProgress: JumpProgress | null;
  finalStats: IpcRunStatistics | null;
  loadedReference: LoadedReference | null;
  theme: ThemeChoice;
  activeView: ActiveView;
  animateTransitions: boolean;
  connected: boolean;
  initError: string | null;
  aggregateRows: AggregateRow[];

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
  setPlayRate: (gps: number) => Promise<void>;
  pause: () => Promise<void>;
  step: () => Promise<void>;
  restart: () => Promise<void>;
  jumpTo: (target: number) => Promise<void>;
  extendMaxIterations: (newTotal: number) => Promise<void>;
  editBoard: () => Promise<void>;

  // Settings
  setTheme: (theme: ThemeChoice) => void;

  // Navigation
  setActiveView: (view: ActiveView) => void;

  // Rendering preferences
  setAnimateTransitions: (value: boolean) => void;

  // Aggregate analysis
  addAggregateFiles: (paths: string[]) => Promise<void>;
  removeAggregateRow: (path: string) => void;
  clearAggregate: () => void;
  setAggregateRowVisible: (path: string, visible: boolean) => void;

  // Persistence
  loadBoardSnapshot: () => Promise<void>;
  loadRunBoard: (selection: RunBoardSelection) => Promise<void>;
  loadSavedRun: () => Promise<void>;
  saveBoardSnapshot: () => Promise<void>;

  // Tear-down
  disconnect: () => void;
}

let unlistens: UnlistenFn[] = [];
const pendingAggregateRequests = new Map<string, symbol>();

const DEFAULT_NEW_RUN: CreateRunArgs = {
  width: 20,
  height: 20,
  source: { kind: "pattern", value: "demo" },
  maxIterations: 100,
};

const messageFromUnknown = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  return String(error);
};

const getFilenameFromPath = (path: string): string => {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
};

const nextColorIndex = (used: Set<number>): number => {
  let index = 0;
  while (used.has(index)) {
    index += 1;
  }
  used.add(index);
  return index;
};

export const useStore = create<AppState>((set, get) => ({
  session: null,
  board: null,
  history: [],
  latestTick: null,
  jumpProgress: null,
  finalStats: null,
  loadedReference: null,
  theme: "light",
  activeView: loadPersistedActiveView(),
  animateTransitions: loadPersistedAnimateTransitions(),
  connected: false,
  initError: null,
  aggregateRows: [],

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
        if (session.completed) {
          await get().refreshFinalStats();
        }
      }

      // Register listeners sequentially so a mid-list failure can still
      // tear down whatever already registered. Promise.all() would leak
      // the earlier-resolved unlisten functions on a later rejection.
      localUnlistens.push(
        await onBoardTick((tick) => {
          const currentSession = get().session;
          const shouldRecordHistory =
            currentSession === null || tick.iteration > currentSession.iteration;
          set({
            session: currentSession
              ? { ...currentSession, iteration: tick.iteration }
              : currentSession,
            board: decodeBoard(tick.board),
            latestTick: {
              iteration: tick.iteration,
              alive: tick.alive,
              dead: tick.dead,
              births: tick.births,
              deaths: tick.deaths,
            },
            history: shouldRecordHistory ? [...get().history, tick.alive] : get().history,
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
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  setCell: async (x, y, alive) => {
    await setCell(x, y, alive);
    await get().refreshBoard();
    await get().refreshSession();
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  paintCells: async (edits) => {
    await paintCells(edits);
    await get().refreshBoard();
    await get().refreshSession();
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  applyPattern: async (pattern) => {
    await applyPattern(pattern);
    await get().refreshBoard();
    await get().refreshSession();
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  randomize: async (seed, aliveCellsPerThousand) => {
    await randomize(seed, aliveCellsPerThousand);
    await get().refreshBoard();
    await get().refreshSession();
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  clearBoard: async () => {
    await clearBoard();
    await get().refreshBoard();
    await get().refreshSession();
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  startRun: async () => {
    await startRun();
    await get().refreshSession();
    set({ history: [], finalStats: null, loadedReference: null });
  },

  play: async (gps) => {
    await play(gps);
    await get().refreshSession();
  },

  setPlayRate: async (gps) => {
    // Pure rate update — no session refresh needed; the backend just
    // writes an atomic and the worker reads it on its next tick.
    await setPlayRate(gps);
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
    set({ finalStats: null, loadedReference: null, latestTick: null });
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
    set({
      history: [],
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  setTheme: (theme) => {
    set({ theme });
  },

  setActiveView: (view) => {
    // Telemetry is reserved as a future destination; the nav-rail tab is
    // disabled, so this branch isn't reachable from normal UI. Tests or
    // external callers that set it will be coerced down to "edit" on the
    // next reload anyway; for symmetry, drop the persisted value here too
    // rather than letting it linger in localStorage.
    persistActiveView(view);
    set({ activeView: view });
  },

  setAnimateTransitions: (value) => {
    persistAnimateTransitions(value);
    set({ animateTransitions: value });
  },

  addAggregateFiles: async (paths) => {
    const existingPaths = new Set(get().aggregateRows.map((row) => row.path));
    const uniqueNewPaths = paths.filter((path, index) => {
      if (existingPaths.has(path) || paths.indexOf(path) !== index) {
        return false;
      }
      return true;
    });
    if (uniqueNewPaths.length === 0) {
      return;
    }

    const usedColors = new Set(get().aggregateRows.map((row) => row.colorIndex));
    let visibleCount = get().aggregateRows.filter((row) => row.visible).length;
    const requestTokens = new Map<string, symbol>();
    const rowsToAdd: AggregateRow[] = uniqueNewPaths.map((path) => {
      const visible = visibleCount < 8;
      if (visible) {
        visibleCount += 1;
      }
      const token = Symbol(path);
      pendingAggregateRequests.set(path, token);
      requestTokens.set(path, token);
      return {
        path,
        filename: getFilenameFromPath(path),
        status: "loading",
        colorIndex: nextColorIndex(usedColors),
        visible,
      };
    });

    set((state) => ({ aggregateRows: [...state.aggregateRows, ...rowsToAdd] }));

    const reads = rowsToAdd.map((row) => readRunSeries(row.path));
    const results = await Promise.allSettled(reads);
    results.forEach((result, index) => {
      const path = rowsToAdd[index].path;
      if (pendingAggregateRequests.get(path) !== requestTokens.get(path)) {
        return;
      }
      pendingAggregateRequests.delete(path);
      if (result.status === "fulfilled") {
        const payload = result.value;
        set((state) => ({
          aggregateRows: state.aggregateRows.map((row) =>
            row.path === path
              ? {
                  ...row,
                  filename: payload.filename || row.filename,
                  status: payload.series ? "ready" : "summaryOnly",
                  series: payload.series ?? undefined,
                  summary: payload.summary,
                  error: undefined,
                }
              : row,
          ),
        }));
        return;
      }

      set((state) => ({
        aggregateRows: state.aggregateRows.map((row) =>
          row.path === path
            ? {
                ...row,
                status: "error",
                error: messageFromUnknown(result.reason),
              }
            : row,
        ),
      }));
    });
  },

  removeAggregateRow: (path) => {
    pendingAggregateRequests.delete(path);
    set((state) => ({
      aggregateRows: state.aggregateRows.filter((row) => row.path !== path),
    }));
  },

  clearAggregate: () => {
    pendingAggregateRequests.clear();
    set({ aggregateRows: [] });
  },

  setAggregateRowVisible: (path, visible) => {
    set((state) => ({
      aggregateRows: state.aggregateRows.map((row) =>
        row.path === path ? { ...row, visible } : row,
      ),
    }));
  },

  loadBoardSnapshot: async () => {
    const { open, ask, message } = await import("@tauri-apps/plugin-dialog");
    const session = get().session;
    if (session?.dirty) {
      const discard = await ask(
        "The current board has unsaved changes. Discard them and load another file?",
        { title: "Discard unsaved changes?", kind: "warning" },
      );
      if (!discard) {
        return;
      }
    }

    const chosen = await open({
      title: "Load board snapshot or run",
      multiple: false,
      filters: [{ name: "Game of Life file", extensions: ["gol"] }],
    });
    if (!chosen) {
      return;
    }
    const path = Array.isArray(chosen) ? chosen[0] : chosen;
    if (!path) {
      return;
    }

    try {
      await loadBoardSnapshot(path);
    } catch (error) {
      await message(messageFromUnknown(error), {
        title: "Unable to load board",
        kind: "error",
      });
      return;
    }

    await get().refreshSession();
    await get().refreshBoard();
    await get().refreshHistory();
    set({
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  loadRunBoard: async (selection) => {
    const { open, ask, message } = await import("@tauri-apps/plugin-dialog");
    const session = get().session;
    if (session?.dirty) {
      const discard = await ask(
        "The current board has unsaved changes. Discard them and load a run?",
        { title: "Discard unsaved changes?", kind: "warning" },
      );
      if (!discard) {
        return;
      }
    }

    const chosen = await open({
      title: `Load ${selection} board from run`,
      multiple: false,
      filters: [{ name: "Game of Life run", extensions: ["gol"] }],
    });
    if (!chosen) {
      return;
    }
    const path = Array.isArray(chosen) ? chosen[0] : chosen;
    if (!path) {
      return;
    }

    try {
      await loadRunBoard(path, selection);
    } catch (error) {
      await message(messageFromUnknown(error), {
        title: "Unable to load run",
        kind: "error",
      });
      return;
    }

    await get().refreshSession();
    await get().refreshBoard();
    await get().refreshHistory();
    set({
      latestTick: null,
      finalStats: null,
      loadedReference: null,
      jumpProgress: null,
    });
  },

  loadSavedRun: async () => {
    const { open, ask, message } = await import("@tauri-apps/plugin-dialog");
    const session = get().session;
    if (session?.dirty) {
      const discard = await ask(
        "The current board has unsaved changes. Discard them and load a saved run?",
        { title: "Discard unsaved changes?", kind: "warning" },
      );
      if (!discard) {
        return;
      }
    }

    const chosen = await open({
      title: "Load saved run",
      multiple: false,
      filters: [{ name: "Game of Life run", extensions: ["gol"] }],
    });
    if (!chosen) {
      return;
    }
    const path = Array.isArray(chosen) ? chosen[0] : chosen;
    if (!path) {
      return;
    }

    let runSeries: Awaited<ReturnType<typeof readRunSeries>>;
    try {
      runSeries = await readRunSeries(path);
    } catch (error) {
      await message(messageFromUnknown(error), {
        title: "Unable to load saved run",
        kind: "error",
      });
      return;
    }

    try {
      await loadRunBoard(path, "initial");
    } catch (error) {
      await message(messageFromUnknown(error), {
        title: "Unable to load saved run",
        kind: "error",
      });
      return;
    }

    await get().refreshSession();
    await get().refreshBoard();
    set({
      history: runSeries.series?.alive ?? [],
      finalStats: runSeries.summary,
      loadedReference: {
        path: runSeries.path,
        filename: runSeries.filename,
        summaryOnly: runSeries.series === null,
      },
      latestTick: null,
      jumpProgress: null,
    });
  },

  saveBoardSnapshot: async () => {
    // Lazy-import so the dialog plugin is only loaded when the user
    // actually triggers a save. Keeps the initial JS bundle smaller
    // and avoids touching the Tauri runtime on smoke tests.
    const { save, ask, message } = await import("@tauri-apps/plugin-dialog");
    const session = get().session;
    if (!session) {
      return;
    }
    const defaultName = `board-iter-${session.iteration}.gol`;
    let defaultPath: string | undefined;
    if (session.savePath) {
      defaultPath = session.savePath;
    } else {
      try {
        defaultPath = `${await defaultSaveDir()}/${defaultName}`;
      } catch {
        defaultPath = defaultName;
      }
    }
    const chosen = await save({
      title: "Save board snapshot",
      defaultPath,
      filters: [{ name: "Game of Life board", extensions: ["gol"] }],
    });
    if (!chosen) {
      return;
    }
    try {
      await saveBoardSnapshot(chosen, false);
    } catch (error) {
      const msg = messageFromUnknown(error);
      if (msg.toLowerCase().includes("refusing to overwrite")) {
        const overwrite = await ask(
          `${chosen} already exists. Overwrite?`,
          { title: "Overwrite file?", kind: "warning" },
        );
        if (overwrite) {
          try {
            await saveBoardSnapshot(chosen, true);
          } catch (overwriteError) {
            await message(messageFromUnknown(overwriteError), {
              title: "Unable to save board",
              kind: "error",
            });
            return;
          }
        } else {
          return;
        }
      } else {
        await message(msg, {
          title: "Unable to save board",
          kind: "error",
        });
        return;
      }
    }
    await get().refreshSession();
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
