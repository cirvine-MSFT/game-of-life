import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { BoardTick, IpcRunSeries, IpcRunStatistics, SessionInfo } from "../../src/ipc";

const dialog = vi.hoisted(() => ({
  open: vi.fn(),
  save: vi.fn(),
  ask: vi.fn(),
  message: vi.fn(),
}));

// We mock the entire IPC module so the store can be exercised without a
// Tauri runtime. The setup file already mocks @tauri-apps/api/core +
// /event, but we need finer control here.
vi.mock("../../src/ipc", async () => {
  const actual =
    await vi.importActual<typeof import("../../src/ipc")>("../../src/ipc");
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
    readRunSeries: vi.fn(async () => null),
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
    loadBoardSnapshot: vi.fn(async () => "/tmp/loaded.gol"),
    loadRunBoard: vi.fn(async () => "/tmp/run.gol"),
    saveBoardSnapshot: vi.fn(async () => "/tmp/test.gol"),
    defaultSaveDir: vi.fn(async () => "/tmp"),
    onBoardTick: vi.fn(async () => () => undefined),
    onJumpProgress: vi.fn(async () => () => undefined),
    onRunCompleted: vi.fn(async () => () => undefined),
    onSessionChanged: vi.fn(async () => () => undefined),
  };
});

vi.mock("@tauri-apps/plugin-dialog", () => dialog);

import * as ipc from "../../src/ipc";
import { useStore, loadPersistedActiveView, loadPersistedAnimateTransitions } from "../../src/state/store";

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

const completedSession: SessionInfo = {
  ...populatedSession,
  mode: "paused",
  iteration: 12,
  completed: true,
  status: "stable",
};

const completedStats: IpcRunStatistics = {
  initialAliveCount: 4,
  finalAliveCount: 4,
  peakAliveCount: 4,
  peakAliveGeneration: 0,
  minAliveCount: 4,
  minAliveGeneration: 0,
  totalBirths: 0,
  totalDeaths: 0,
  iterationsRun: 12,
  status: "stable",
  cycleStartGeneration: null,
  cycleDetectedGeneration: null,
  cyclePeriod: null,
};

const savedRunSeries: IpcRunSeries = {
  path: "C:\\runs\\saved.gol",
  filename: "saved.gol",
  summary: completedStats,
  series: {
    alive: [4, 6, 5],
    births: [0, 3, 1],
    deaths: [0, 1, 2],
  },
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
    loadedReference: null,
    theme: "light",
    activeView: "edit",
    animateTransitions: true,
    connected: false,
    initError: null,
    aggregateRows: [],
  });
  try {
    localStorage.clear();
  } catch {
    // jsdom may throw in restricted modes; ignore.
  }
};

beforeEach(() => {
  resetStore();
  vi.clearAllMocks();
  dialog.open.mockResolvedValue(null);
  dialog.save.mockResolvedValue(null);
  dialog.ask.mockResolvedValue(true);
  dialog.message.mockResolvedValue(undefined);
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

  it("loads final stats when reconnecting to a completed session", async () => {
    vi.mocked(ipc.getSession).mockResolvedValueOnce(completedSession);
    vi.mocked(ipc.getFinalStats).mockResolvedValueOnce(completedStats);

    await useStore.getState().connect();

    expect(ipc.getBoard).toHaveBeenCalledTimes(1);
    expect(ipc.getAliveHistory).toHaveBeenCalledTimes(1);
    expect(ipc.getFinalStats).toHaveBeenCalledTimes(1);
    expect(useStore.getState().finalStats).toEqual(completedStats);
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

describe("setActiveView()", () => {
  it("updates the active view and persists known destinations to localStorage", () => {
    useStore.getState().setActiveView("aggregate");
    expect(useStore.getState().activeView).toBe("aggregate");
    expect(localStorage.getItem("gol.activeView")).toBe("aggregate");

    useStore.getState().setActiveView("run");
    expect(localStorage.getItem("gol.activeView")).toBe("run");
  });

  it("does not persist the telemetry destination so a coerced reload lands on edit", () => {
    // Simulate a future caller wiring up telemetry; we don't expose this in
    // the rail today, but the store still accepts it.
    useStore.getState().setActiveView("aggregate");
    useStore.getState().setActiveView("telemetry");

    expect(useStore.getState().activeView).toBe("telemetry");
    expect(localStorage.getItem("gol.activeView")).toBeNull();
  });
});

describe("loadPersistedActiveView()", () => {
  it("returns the persisted value when it is a recognized destination", () => {
    localStorage.setItem("gol.activeView", "aggregate");
    expect(loadPersistedActiveView()).toBe("aggregate");
  });

  it("coerces the telemetry destination to edit on load", () => {
    localStorage.setItem("gol.activeView", "telemetry");
    expect(loadPersistedActiveView()).toBe("edit");
  });

  it("coerces unknown destinations to edit", () => {
    localStorage.setItem("gol.activeView", "not-a-pane");
    expect(loadPersistedActiveView()).toBe("edit");
  });

  it("defaults to edit when nothing is persisted", () => {
    localStorage.removeItem("gol.activeView");
    expect(loadPersistedActiveView()).toBe("edit");
  });
});

describe("setAnimateTransitions()", () => {
  it("updates the store value and persists it to localStorage", () => {
    expect(useStore.getState().animateTransitions).toBe(true);

    useStore.getState().setAnimateTransitions(false);
    expect(useStore.getState().animateTransitions).toBe(false);
    expect(localStorage.getItem("gol.animateTransitions")).toBe("false");

    useStore.getState().setAnimateTransitions(true);
    expect(localStorage.getItem("gol.animateTransitions")).toBe("true");
  });
});

describe("loadPersistedAnimateTransitions()", () => {
  it("returns false when explicitly disabled", () => {
    localStorage.setItem("gol.animateTransitions", "false");
    expect(loadPersistedAnimateTransitions()).toBe(false);
  });

  it("returns true when explicitly enabled", () => {
    localStorage.setItem("gol.animateTransitions", "true");
    expect(loadPersistedAnimateTransitions()).toBe(true);
  });

  it("defaults to true (animations on) when nothing is persisted", () => {
    localStorage.removeItem("gol.animateTransitions");
    expect(loadPersistedAnimateTransitions()).toBe(true);
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
      loadedReference: {
        path: "C:\\runs\\reference.gol",
        filename: "reference.gol",
        summaryOnly: false,
      },
      jumpProgress: { current: 1, target: 10 },
    });

    await useStore.getState().editBoard();

    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().latestTick).toBeNull();
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().loadedReference).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
    expect(ipc.editBoard).toHaveBeenCalledTimes(1);
  });
});

describe("setup mutations", () => {
  it("clear loaded-reference state once the setup board changes", async () => {
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({
      session: populatedSession,
      history: [4, 6, 5],
      latestTick: { iteration: 2, alive: 5, dead: 7, births: 1, deaths: 2 },
      finalStats: completedStats,
      loadedReference: {
        path: "C:\\runs\\reference.gol",
        filename: "reference.gol",
        summaryOnly: false,
      },
      jumpProgress: { current: 1, target: 2 },
    });

    await useStore.getState().setCell(1, 1, true);

    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().latestTick).toBeNull();
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().loadedReference).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
  });
});

describe("startRun()", () => {
  it("clears reference data and refreshes session", async () => {
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({
      history: [4, 5, 6],
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
      loadedReference: {
        path: "C:\\runs\\reference.gol",
        filename: "reference.gol",
        summaryOnly: false,
      },
    });

    await useStore.getState().startRun();

    expect(ipc.startRun).toHaveBeenCalledTimes(1);
    expect(ipc.getSession).toHaveBeenCalled();
    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().loadedReference).toBeNull();
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
      loadedReference: {
        path: "C:\\runs\\reference.gol",
        filename: "reference.gol",
        summaryOnly: false,
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
    expect(useStore.getState().loadedReference).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
  });
});

describe("loadBoardSnapshot()", () => {
  it("loads a selected snapshot and refreshes session state", async () => {
    dialog.open.mockResolvedValueOnce("/tmp/loaded.gol");
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({
      session: populatedSession,
      history: [1, 2],
      latestTick: { iteration: 2, alive: 3, dead: 6, births: 1, deaths: 0 },
      finalStats: {
        initialAliveCount: 1,
        finalAliveCount: 3,
        peakAliveCount: 3,
        peakAliveGeneration: 2,
        minAliveCount: 1,
        minAliveGeneration: 0,
        totalBirths: 2,
        totalDeaths: 0,
        iterationsRun: 2,
        status: "maxIterations",
      },
      loadedReference: {
        path: "C:\\runs\\reference.gol",
        filename: "reference.gol",
        summaryOnly: false,
      },
      jumpProgress: { current: 2, target: 10 },
    });

    await useStore.getState().loadBoardSnapshot();

    expect(dialog.open).toHaveBeenCalledWith({
      title: "Load board snapshot or run",
      multiple: false,
      filters: [{ name: "Game of Life file", extensions: ["gol"] }],
    });
    expect(ipc.loadBoardSnapshot).toHaveBeenCalledWith("/tmp/loaded.gol");
    expect(ipc.getSession).toHaveBeenCalled();
    expect(ipc.getBoard).toHaveBeenCalled();
    expect(ipc.getAliveHistory).toHaveBeenCalled();
    expect(useStore.getState().latestTick).toBeNull();
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().loadedReference).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
  });

  it("does nothing when the file picker is cancelled", async () => {
    useStore.setState({ session: populatedSession });

    await useStore.getState().loadBoardSnapshot();

    expect(ipc.loadBoardSnapshot).not.toHaveBeenCalled();
    expect(ipc.getBoard).not.toHaveBeenCalled();
  });

  it("does nothing when dirty-board discard is cancelled", async () => {
    dialog.ask.mockResolvedValueOnce(false);
    useStore.setState({ session: { ...populatedSession, dirty: true } });

    await useStore.getState().loadBoardSnapshot();

    expect(dialog.ask).toHaveBeenCalled();
    expect(dialog.open).not.toHaveBeenCalled();
    expect(ipc.loadBoardSnapshot).not.toHaveBeenCalled();
  });

  it("shows invalid file errors without refreshing state", async () => {
    dialog.open.mockResolvedValueOnce("/tmp/bad.gol");
    vi.mocked(ipc.loadBoardSnapshot).mockRejectedValueOnce({
      kind: "loadBoardSnapshot",
      message: "Board grid contains unknown character",
    });
    useStore.setState({ session: populatedSession });

    await useStore.getState().loadBoardSnapshot();

    expect(dialog.message).toHaveBeenCalledWith("Board grid contains unknown character", {
      title: "Unable to load board",
      kind: "error",
    });
    expect(ipc.getSession).not.toHaveBeenCalled();
    expect(ipc.getBoard).not.toHaveBeenCalled();
  });
});

describe("loadRunBoard()", () => {
  it("loads the selected board from a run record", async () => {
    dialog.open.mockResolvedValueOnce("/tmp/run.gol");
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({
      session: populatedSession,
      history: [1, 2],
      latestTick: { iteration: 2, alive: 3, dead: 6, births: 1, deaths: 0 },
      finalStats: {
        initialAliveCount: 1,
        finalAliveCount: 3,
        peakAliveCount: 3,
        peakAliveGeneration: 2,
        minAliveCount: 1,
        minAliveGeneration: 0,
        totalBirths: 2,
        totalDeaths: 0,
        iterationsRun: 2,
        status: "maxIterations",
      },
      loadedReference: {
        path: "C:\\runs\\reference.gol",
        filename: "reference.gol",
        summaryOnly: false,
      },
      jumpProgress: { current: 2, target: 10 },
    });

    await useStore.getState().loadRunBoard("final");

    expect(dialog.open).toHaveBeenCalledWith({
      title: "Load final board from run",
      multiple: false,
      filters: [{ name: "Game of Life run", extensions: ["gol"] }],
    });
    expect(ipc.loadRunBoard).toHaveBeenCalledWith("/tmp/run.gol", "final");
    expect(ipc.getSession).toHaveBeenCalled();
    expect(ipc.getBoard).toHaveBeenCalled();
    expect(ipc.getAliveHistory).toHaveBeenCalled();
    expect(useStore.getState().latestTick).toBeNull();
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().loadedReference).toBeNull();
    expect(useStore.getState().jumpProgress).toBeNull();
  });

  it("shows run load errors without refreshing state", async () => {
    dialog.open.mockResolvedValueOnce("/tmp/bad-run.gol");
    vi.mocked(ipc.loadRunBoard).mockRejectedValueOnce({
      kind: "loadRunRecord",
      message: "Selected file is not a valid Game of Life run record",
    });
    useStore.setState({ session: populatedSession });

    await useStore.getState().loadRunBoard("initial");

    expect(dialog.message).toHaveBeenCalledWith(
      "Selected file is not a valid Game of Life run record",
      {
        title: "Unable to load run",
        kind: "error",
      },
    );
    expect(ipc.getSession).not.toHaveBeenCalled();
    expect(ipc.getBoard).not.toHaveBeenCalled();
  });
});

describe("loadSavedRun()", () => {
  it("loads reference series before mutating the backend session", async () => {
    dialog.open.mockResolvedValueOnce("C:\\runs\\saved.gol");
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce(savedRunSeries);
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({ session: populatedSession });

    await useStore.getState().loadSavedRun();

    expect(dialog.open).toHaveBeenCalledWith({
      title: "Load saved run",
      multiple: false,
      filters: [{ name: "Game of Life run", extensions: ["gol"] }],
    });
    expect(ipc.readRunSeries).toHaveBeenCalledWith("C:\\runs\\saved.gol");
    expect(ipc.loadRunBoard).toHaveBeenCalledWith("C:\\runs\\saved.gol", "initial");
    expect(useStore.getState().history).toEqual([4, 6, 5]);
    expect(useStore.getState().finalStats).toEqual(completedStats);
    expect(useStore.getState().loadedReference).toEqual({
      path: "C:\\runs\\saved.gol",
      filename: "saved.gol",
      summaryOnly: false,
    });
  });

  it("shows read errors without mutating session state", async () => {
    dialog.open.mockResolvedValueOnce("C:\\runs\\bad.gol");
    vi.mocked(ipc.readRunSeries).mockRejectedValueOnce({
      kind: "loadRunRecord",
      message: "content hash mismatch",
    });
    useStore.setState({
      session: populatedSession,
      history: [1, 2],
      finalStats: completedStats,
    });

    await useStore.getState().loadSavedRun();

    expect(dialog.message).toHaveBeenCalledWith("content hash mismatch", {
      title: "Unable to load saved run",
      kind: "error",
    });
    expect(ipc.loadRunBoard).not.toHaveBeenCalled();
    expect(ipc.getSession).not.toHaveBeenCalled();
    expect(useStore.getState().history).toEqual([1, 2]);
    expect(useStore.getState().finalStats).toEqual(completedStats);
    expect(useStore.getState().loadedReference).toBeNull();
  });

  it("marks v1 run records as summary-only", async () => {
    dialog.open.mockResolvedValueOnce("C:\\runs\\legacy.gol");
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce({
      ...savedRunSeries,
      path: "C:\\runs\\legacy.gol",
      filename: "legacy.gol",
      series: null,
    });
    vi.mocked(ipc.getSession).mockResolvedValue(populatedSession);
    useStore.setState({ session: populatedSession });

    await useStore.getState().loadSavedRun();

    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().finalStats).toEqual(completedStats);
    expect(useStore.getState().loadedReference).toEqual({
      path: "C:\\runs\\legacy.gol",
      filename: "legacy.gol",
      summaryOnly: true,
    });
  });
});


const aggregateSeriesFor = (path: string, filename: string): IpcRunSeries => ({
  path,
  filename,
  summary: completedStats,
  series: {
    alive: [4, 5, 6],
    births: [0, 2, 2],
    deaths: [0, 1, 1],
  },
});

describe("aggregate slice", () => {
  it("addAggregateFiles dedupes by path", async () => {
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce(
      aggregateSeriesFor("C:\\runs\\a.gol", "a.gol"),
    );

    await useStore.getState().addAggregateFiles([
      "C:\\runs\\a.gol",
      "C:\\runs\\a.gol",
    ]);
    await useStore.getState().addAggregateFiles(["C:\\runs\\a.gol"]);

    expect(ipc.readRunSeries).toHaveBeenCalledTimes(1);
    expect(useStore.getState().aggregateRows).toHaveLength(1);
  });

  it("addAggregateFiles patches a ready row with series and summary", async () => {
    const payload = aggregateSeriesFor("C:\\runs\\ready.gol", "ready.gol");
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce(payload);

    await useStore.getState().addAggregateFiles(["C:\\runs\\ready.gol"]);

    expect(useStore.getState().aggregateRows[0]).toMatchObject({
      path: "C:\\runs\\ready.gol",
      filename: "ready.gol",
      status: "ready",
      summary: completedStats,
      series: payload.series,
    });
  });

  it("addAggregateFiles patches a v1 file as summaryOnly", async () => {
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce({
      ...aggregateSeriesFor("C:\\runs\\legacy.gol", "legacy.gol"),
      series: null,
    });

    await useStore.getState().addAggregateFiles(["C:\\runs\\legacy.gol"]);

    expect(useStore.getState().aggregateRows[0]).toMatchObject({
      status: "summaryOnly",
      summary: completedStats,
      series: undefined,
    });
  });

  it("addAggregateFiles patches a rejected read as error", async () => {
    vi.mocked(ipc.readRunSeries).mockRejectedValueOnce(new Error("bad magic"));

    await useStore.getState().addAggregateFiles(["C:\\runs\\bad.gol"]);

    expect(useStore.getState().aggregateRows[0]).toMatchObject({
      path: "C:\\runs\\bad.gol",
      status: "error",
      error: "bad magic",
    });
  });

  it("ignores stale reads after a row is removed and re-added", async () => {
    let resolveFirst: (payload: IpcRunSeries) => void = () => undefined;
    const firstRead = new Promise<IpcRunSeries>((resolve) => {
      resolveFirst = resolve;
    });
    vi.mocked(ipc.readRunSeries)
      .mockReturnValueOnce(firstRead)
      .mockResolvedValueOnce(
        aggregateSeriesFor("C:\\runs\\same.gol", "new.gol"),
      );

    const firstAdd = useStore.getState().addAggregateFiles(["C:\\runs\\same.gol"]);
    useStore.getState().removeAggregateRow("C:\\runs\\same.gol");
    const secondAdd = useStore.getState().addAggregateFiles(["C:\\runs\\same.gol"]);

    await secondAdd;
    resolveFirst(aggregateSeriesFor("C:\\runs\\same.gol", "old.gol"));
    await firstAdd;

    expect(useStore.getState().aggregateRows).toHaveLength(1);
    expect(useStore.getState().aggregateRows[0]).toMatchObject({
      filename: "new.gol",
      status: "ready",
    });
  });

  it("removeAggregateRow removes the matching row", () => {
    useStore.setState({
      aggregateRows: [
        {
          path: "C:\\runs\\a.gol",
          filename: "a.gol",
          status: "ready",
          colorIndex: 0,
          visible: true,
          summary: completedStats,
          series: aggregateSeriesFor("C:\\runs\\a.gol", "a.gol").series ?? undefined,
        },
        {
          path: "C:\\runs\\b.gol",
          filename: "b.gol",
          status: "error",
          colorIndex: 1,
          visible: false,
          error: "bad magic",
        },
      ],
    });

    useStore.getState().removeAggregateRow("C:\\runs\\a.gol");

    expect(useStore.getState().aggregateRows.map((row) => row.path)).toEqual([
      "C:\\runs\\b.gol",
    ]);
  });

  it("clearAggregate empties the list", () => {
    useStore.setState({
      aggregateRows: [
        {
          path: "C:\\runs\\a.gol",
          filename: "a.gol",
          status: "ready",
          colorIndex: 0,
          visible: true,
        },
      ],
    });

    useStore.getState().clearAggregate();

    expect(useStore.getState().aggregateRows).toEqual([]);
  });

  it("setAggregateRowVisible flips visibility for the matching row", () => {
    useStore.setState({
      aggregateRows: [
        {
          path: "C:\\runs\\a.gol",
          filename: "a.gol",
          status: "ready",
          colorIndex: 0,
          visible: true,
        },
      ],
    });

    useStore.getState().setAggregateRowVisible("C:\\runs\\a.gol", false);

    expect(useStore.getState().aggregateRows[0].visible).toBe(false);
  });
});

describe("saveBoardSnapshot()", () => {
  it("defaults to the current save path when one exists", async () => {
    dialog.save.mockResolvedValueOnce(null);
    useStore.setState({
      session: { ...populatedSession, savePath: "/tmp/current.gol" },
    });

    await useStore.getState().saveBoardSnapshot();

    expect(dialog.save).toHaveBeenCalledWith({
      title: "Save board snapshot",
      defaultPath: "/tmp/current.gol",
      filters: [{ name: "Game of Life board", extensions: ["gol"] }],
    });
    expect(ipc.defaultSaveDir).not.toHaveBeenCalled();
    expect(ipc.saveBoardSnapshot).not.toHaveBeenCalled();
  });

  it("shows non-overwrite save errors", async () => {
    dialog.save.mockResolvedValueOnce("/tmp/current.gol");
    vi.mocked(ipc.saveBoardSnapshot).mockRejectedValueOnce({
      kind: "saveBoardSnapshot",
      message: "disk full",
    });
    useStore.setState({ session: populatedSession });

    await useStore.getState().saveBoardSnapshot();

    expect(dialog.message).toHaveBeenCalledWith("disk full", {
      title: "Unable to save board",
      kind: "error",
    });
    expect(ipc.getSession).not.toHaveBeenCalled();
  });

  it("shows overwrite retry save errors", async () => {
    dialog.save.mockResolvedValueOnce("/tmp/current.gol");
    vi.mocked(ipc.saveBoardSnapshot)
      .mockRejectedValueOnce({
        kind: "saveBoardSnapshot",
        message: "refusing to overwrite existing file",
      })
      .mockRejectedValueOnce({
        kind: "saveBoardSnapshot",
        message: "permission denied",
      });
    useStore.setState({ session: populatedSession });

    await useStore.getState().saveBoardSnapshot();

    expect(dialog.ask).toHaveBeenCalledWith(
      "/tmp/current.gol already exists. Overwrite?",
      { title: "Overwrite file?", kind: "warning" },
    );
    expect(dialog.message).toHaveBeenCalledWith("permission denied", {
      title: "Unable to save board",
      kind: "error",
    });
    expect(ipc.getSession).not.toHaveBeenCalled();
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
