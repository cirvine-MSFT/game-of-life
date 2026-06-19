import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { BoardTick, SessionInfo } from "../ipc";

// We mock the entire IPC module so the store can be exercised without a
// Tauri runtime. The setup file already mocks @tauri-apps/api/core +
// /event, but we need finer control here.
vi.mock("../ipc", async () => {
  const actual =
    await vi.importActual<typeof import("../ipc")>("../ipc");
  return {
    ...actual,
    getSession: vi.fn(),
    getBoard: vi.fn(async () => ({
      width: 0,
      height: 0,
      iteration: 0,
      cellsBase64: "",
    })),
    getAliveHistory: vi.fn(async () => []),
    getFinalStats: vi.fn(async () => null),
    createRun: vi.fn(async () => undefined),
    setCell: vi.fn(async () => undefined),
    paintCells: vi.fn(async () => undefined),
    applyPattern: vi.fn(async () => undefined),
    randomize: vi.fn(async () => undefined),
    clearBoard: vi.fn(async () => undefined),
    startRun: vi.fn(async () => undefined),
    play: vi.fn(async () => undefined),
    pause: vi.fn(async () => undefined),
    step: vi.fn(async () => undefined),
    restart: vi.fn(async () => undefined),
    jumpTo: vi.fn(async () => undefined),
    extendMaxIterations: vi.fn(async () => undefined),
    editBoard: vi.fn(async () => undefined),
    saveBoardSnapshot: vi.fn(async () => "/tmp/test.gol"),
    defaultSaveDir: vi.fn(async () => "/tmp"),
    onBoardTick: vi.fn(async () => () => undefined),
    onJumpProgress: vi.fn(async () => () => undefined),
    onRunCompleted: vi.fn(async () => () => undefined),
    onSessionChanged: vi.fn(async () => () => undefined),
  };
});

import * as ipc from "../ipc";
import { useStore } from "./store";

const setupSession: SessionInfo = {
  mode: "setup",
  iteration: 0,
  width: 0,
  height: 0,
  maxIterations: 0,
  savePath: null,
  dirty: false,
  completed: false,
  jumpTarget: null,
  status: null,
};

const populatedSession: SessionInfo = {
  mode: "setup",
  iteration: 0,
  width: 20,
  height: 20,
  maxIterations: 100,
  savePath: null,
  dirty: false,
  completed: false,
  jumpTarget: null,
  status: null,
};

const resetStore = () => {
  // Zustand stores are module-singletons; reset between tests so
  // ordering doesn't matter.
  useStore.setState({
    session: null,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    theme: "light",
    connected: false,
    initError: null,
  });
};

beforeEach(() => {
  resetStore();
  vi.clearAllMocks();
});

afterEach(() => {
  resetStore();
});

describe("connect()", () => {
  it("calls newRun with default args when the session has no board", async () => {
    vi.mocked(ipc.getSession).mockResolvedValueOnce(setupSession);
    vi.mocked(ipc.getSession).mockResolvedValueOnce(populatedSession);

    await useStore.getState().connect();

    expect(ipc.createRun).toHaveBeenCalledTimes(1);
    expect(useStore.getState().connected).toBe(true);
    expect(useStore.getState().initError).toBeNull();
  });

  it("skips newRun and refreshes state when a session already exists", async () => {
    vi.mocked(ipc.getSession).mockResolvedValueOnce(populatedSession);

    await useStore.getState().connect();

    expect(ipc.createRun).not.toHaveBeenCalled();
    expect(ipc.getBoard).toHaveBeenCalledTimes(1);
    expect(ipc.getAliveHistory).toHaveBeenCalledTimes(1);
  });

  it("records initError and stays disconnected on getSession failure", async () => {
    vi.mocked(ipc.getSession).mockRejectedValueOnce(new Error("backend down"));

    await useStore.getState().connect();

    expect(useStore.getState().connected).toBe(false);
    expect(useStore.getState().initError).toContain("backend down");
  });

  it("is idempotent: a second connect() while connected is a no-op", async () => {
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);

    await useStore.getState().connect();
    const callsBefore = vi.mocked(ipc.getSession).mock.calls.length;

    await useStore.getState().connect();
    const callsAfter = vi.mocked(ipc.getSession).mock.calls.length;

    expect(callsAfter).toBe(callsBefore);
  });

  it("appends history and advances session iteration for generation ticks", async () => {
    let boardTickHandler: ((tick: BoardTick) => void) | undefined;
    vi.mocked(ipc.getSession).mockResolvedValueOnce(populatedSession);
    vi.mocked(ipc.getAliveHistory).mockResolvedValueOnce([3]);
    vi.mocked(ipc.onBoardTick).mockImplementationOnce(async (handler) => {
      boardTickHandler = handler;
      return () => undefined;
    });

    await useStore.getState().connect();
    boardTickHandler?.({
      iteration: 1,
      alive: 4,
      dead: 5,
      births: 1,
      deaths: 0,
      board: { width: 3, height: 3, iteration: 1, cellsBase64: "AQEAAQAAAAAAAA==" },
    });

    expect(useStore.getState().session?.iteration).toBe(1);
    expect(useStore.getState().history).toEqual([3, 4]);
  });

  it("does not append history for stable confirmation ticks at the current iteration", async () => {
    let boardTickHandler: ((tick: BoardTick) => void) | undefined;
    vi.mocked(ipc.getSession).mockResolvedValueOnce(populatedSession);
    vi.mocked(ipc.getAliveHistory).mockResolvedValueOnce([4]);
    vi.mocked(ipc.onBoardTick).mockImplementationOnce(async (handler) => {
      boardTickHandler = handler;
      return () => undefined;
    });

    await useStore.getState().connect();
    boardTickHandler?.({
      iteration: 0,
      alive: 4,
      dead: 0,
      births: 0,
      deaths: 0,
      board: { width: 2, height: 2, iteration: 0, cellsBase64: "AQEBAQ==" },
    });

    expect(useStore.getState().session?.iteration).toBe(0);
    expect(useStore.getState().latestTick).toMatchObject({ iteration: 0, alive: 4 });
    expect(useStore.getState().history).toEqual([4]);
  });
});

describe("setTheme()", () => {
  it("sets the chosen theme and never touches IPC", async () => {
    useStore.getState().setTheme("dark");
    expect(useStore.getState().theme).toBe("dark");
    expect(ipc.getSession).not.toHaveBeenCalled();
  });
});

describe("editBoard()", () => {
  it("clears history, latestTick, finalStats, and jumpProgress", async () => {
    vi.mocked(ipc.getSession).mockResolvedValue(setupSession);
    useStore.setState({
      history: [1, 2, 3],
      latestTick: { iteration: 3, alive: 5, dead: 5, births: 1, deaths: 0 },
      finalStats: {
        initialAliveCount: 1,
        finalAliveCount: 0,
        peakAliveCount: 5,
        peakAliveGeneration: 2,
        minAliveCount: 0,
        minAliveGeneration: 3,
        totalBirths: 4,
        totalDeaths: 5,
        iterationsRun: 3,
        status: "extinct",
      },
      jumpProgress: { current: 1, target: 10 },
    });

    await useStore.getState().editBoard();

    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().latestTick).toBeNull();
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
    expect(ipc.editBoard).toHaveBeenCalledTimes(1);
  });
});

describe("startRun()", () => {
  it("clears finalStats and refreshes session + history", async () => {
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({
      finalStats: {
        initialAliveCount: 1,
        finalAliveCount: 0,
        peakAliveCount: 5,
        peakAliveGeneration: 2,
        minAliveCount: 0,
        minAliveGeneration: 3,
        totalBirths: 4,
        totalDeaths: 5,
        iterationsRun: 3,
        status: "extinct",
      },
    });

    await useStore.getState().startRun();

    expect(ipc.startRun).toHaveBeenCalledTimes(1);
    expect(ipc.getAliveHistory).toHaveBeenCalled();
    expect(useStore.getState().finalStats).toBeNull();
  });
});

describe("newRun()", () => {
  it("resets transient state on a fresh run", async () => {
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({
      history: [1, 2, 3],
      latestTick: { iteration: 3, alive: 5, dead: 5, births: 1, deaths: 0 },
      finalStats: {
        initialAliveCount: 1,
        finalAliveCount: 0,
        peakAliveCount: 5,
        peakAliveGeneration: 2,
        minAliveCount: 0,
        minAliveGeneration: 3,
        totalBirths: 4,
        totalDeaths: 5,
        iterationsRun: 3,
        status: "extinct",
      },
      jumpProgress: { current: 1, target: 10 },
    });

    await useStore.getState().newRun({
      width: 10,
      height: 10,
      source: { kind: "empty" },
      maxIterations: 50,
    });

    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().latestTick).toBeNull();
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
  });
});

describe("disconnect()", () => {
  it("flips connected back to false and is safe to call when not connected", () => {
    useStore.setState({ connected: true });
    useStore.getState().disconnect();
    expect(useStore.getState().connected).toBe(false);

    // Second call must not throw.
    expect(() => useStore.getState().disconnect()).not.toThrow();
  });
});
