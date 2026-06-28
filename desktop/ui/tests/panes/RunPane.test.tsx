import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { IpcRunSeries, IpcRunStatistics, SessionInfo } from "../../src/ipc";

const dialog = vi.hoisted(() => ({
  open: vi.fn(),
  ask: vi.fn(),
  message: vi.fn(),
}));

vi.mock("../../src/ipc", async () => {
  const actual =
    await vi.importActual<typeof import("../../src/ipc")>("../../src/ipc");
  return {
    ...actual,
    getSession: vi.fn(),
    getBoard: vi.fn(async () => ({
      width: 4,
      height: 3,
      iteration: 0,
      cellsBase64: "AAAAAAAAAAAA",
    })),
    getAliveHistory: vi.fn(async () => []),
    getFinalStats: vi.fn(async () => null),
    readRunSeries: vi.fn(),
    loadBoardSnapshot: vi.fn(async () => "C:\\runs\\board.gol"),
    loadRunBoard: vi.fn(async () => "C:\\runs\\saved.gol"),
    startRun: vi.fn(async () => undefined),
    onBoardTick: vi.fn(async () => () => undefined),
    onJumpProgress: vi.fn(async () => () => undefined),
    onRunCompleted: vi.fn(async () => () => undefined),
    onSessionChanged: vi.fn(async () => () => undefined),
  };
});

vi.mock("@tauri-apps/plugin-dialog", () => dialog);

import * as ipc from "../../src/ipc";
import { RunPane } from "../../src/panes/RunPane";
import { useStore } from "../../src/state/store";

const baseSession: SessionInfo = {
  mode: "setup",
  iteration: 0,
  width: 4,
  height: 3,
  maxIterations: 100,
  savePath: null,
  dirty: false,
  completed: false,
  jumpTarget: null,
  status: null,
};

const completedStats: IpcRunStatistics = {
  initialAliveCount: 4,
  finalAliveCount: 5,
  peakAliveCount: 6,
  peakAliveGeneration: 1,
  minAliveCount: 4,
  minAliveGeneration: 0,
  totalBirths: 4,
  totalDeaths: 3,
  iterationsRun: 2,
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

const resetStore = (session: SessionInfo | null = baseSession) => {
  useStore.setState({
    session,
    board: session
      ? {
          width: session.width,
          height: session.height,
          iteration: session.iteration,
          cells: new Uint8Array(session.width * session.height),
        }
      : null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    loadedReference: null,
    theme: "light",
    activeView: "run",
    connected: false,
    initError: null,
  });
};

beforeEach(() => {
  resetStore();
  vi.clearAllMocks();
  dialog.open.mockResolvedValue(null);
  dialog.ask.mockResolvedValue(true);
  dialog.message.mockResolvedValue(undefined);
  vi.mocked(ipc.getSession).mockResolvedValue(baseSession);
});

afterEach(() => {
  vi.restoreAllMocks();
  resetStore();
});

describe("RunPane", () => {
  it("renders toolbar buttons and calls their store actions", async () => {
    const user = userEvent.setup();
    const loadBoardSnapshot = vi
      .spyOn(useStore.getState(), "loadBoardSnapshot")
      .mockResolvedValue(undefined);
    const loadSavedRun = vi
      .spyOn(useStore.getState(), "loadSavedRun")
      .mockResolvedValue(undefined);
    const loadRunBoard = vi
      .spyOn(useStore.getState(), "loadRunBoard")
      .mockResolvedValue(undefined);

    render(<RunPane />);

    expect(screen.getByRole("toolbar", { name: "Run file toolbar" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Load board" }));
    await user.click(screen.getByRole("button", { name: "Load saved run" }));
    await user.click(screen.getByRole("button", { name: "Import final board…" }));

    expect(loadBoardSnapshot).toHaveBeenCalledTimes(1);
    expect(loadSavedRun).toHaveBeenCalledTimes(1);
    expect(loadRunBoard).toHaveBeenCalledWith("final");
  });

  it("loadSavedRun pre-fills history, finalStats, and loadedReference", async () => {
    dialog.open.mockResolvedValueOnce("C:\\runs\\saved.gol");
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce(savedRunSeries);

    await useStore.getState().loadSavedRun();

    expect(useStore.getState().history).toEqual([4, 6, 5]);
    expect(useStore.getState().finalStats).toEqual(completedStats);
    expect(useStore.getState().loadedReference).toEqual({
      path: "C:\\runs\\saved.gol",
      filename: "saved.gol",
      summaryOnly: false,
    });
  });

  it("marks v1 files as summary-only and renders the banner", async () => {
    dialog.open.mockResolvedValueOnce("C:\\runs\\legacy.gol");
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce({
      ...savedRunSeries,
      path: "C:\\runs\\legacy.gol",
      filename: "legacy.gol",
      series: null,
    });

    await useStore.getState().loadSavedRun();
    render(<RunPane />);

    expect(useStore.getState().loadedReference).toEqual({
      path: "C:\\runs\\legacy.gol",
      filename: "legacy.gol",
      summaryOnly: true,
    });
    expect(screen.getByText("Loaded reference: legacy.gol")).toBeInTheDocument();
    expect(
      screen.getByText("Summary-only run — re-run to capture per-generation data."),
    ).toBeInTheDocument();
  });

  it("startRun clears the loaded reference and history", async () => {
    useStore.setState({
      history: [4, 6, 5],
      finalStats: completedStats,
      loadedReference: {
        path: "C:\\runs\\saved.gol",
        filename: "saved.gol",
        summaryOnly: false,
      },
    });

    await useStore.getState().startRun();

    expect(useStore.getState().history).toEqual([]);
    expect(useStore.getState().finalStats).toBeNull();
    expect(useStore.getState().loadedReference).toBeNull();
  });

  it("passes read-only state to BoardCanvas when the session is not setup", () => {
    resetStore({ ...baseSession, mode: "playing", iteration: 1 });

    render(<RunPane />);

    expect(
      screen.getByRole("img", { name: "Game of Life board 4 by 3" }),
    ).toHaveAttribute("data-readonly", "true");
  });

  it("does not mutate state when the saved-run picker is cancelled", async () => {
    useStore.setState({ history: [1, 2], finalStats: completedStats });

    await useStore.getState().loadSavedRun();

    expect(ipc.readRunSeries).not.toHaveBeenCalled();
    expect(ipc.loadRunBoard).not.toHaveBeenCalled();
    expect(useStore.getState().history).toEqual([1, 2]);
    expect(useStore.getState().finalStats).toEqual(completedStats);
    expect(useStore.getState().loadedReference).toBeNull();
  });

  it("shows read errors and never imports the run board", async () => {
    dialog.open.mockResolvedValueOnce("C:\\runs\\bad.gol");
    vi.mocked(ipc.readRunSeries).mockRejectedValueOnce(new Error("parse failed"));

    await useStore.getState().loadSavedRun();

    await waitFor(() => {
      expect(dialog.message).toHaveBeenCalledWith("parse failed", {
        title: "Unable to load saved run",
        kind: "error",
      });
    });
    expect(ipc.loadRunBoard).not.toHaveBeenCalled();
    expect(useStore.getState().loadedReference).toBeNull();
  });

  describe("Max iterations field", () => {
    it("strips non-digit characters typed into the input", async () => {
      const user = userEvent.setup();
      resetStore(baseSession);
      const extendMaxIterations = vi.fn().mockResolvedValue(undefined);
      useStore.setState({ extendMaxIterations });

      render(<RunPane />);

      const input = screen.getByLabelText("Max iterations");
      await user.clear(input);
      // user.type treats some characters as special (e.g. {Enter}); use
      // distinct literal non-digits that the parser will reject. The
      // onChange filter strips them so the field only holds 12345.
      await user.type(input, "1a2b3c4d5");
      await user.tab();

      expect(extendMaxIterations).toHaveBeenCalledWith(12345);
    });

    it("commits a new value to extendMaxIterations when the input loses focus", async () => {
      const user = userEvent.setup();
      resetStore(baseSession);
      const extendMaxIterations = vi.fn().mockResolvedValue(undefined);
      useStore.setState({ extendMaxIterations });

      render(<RunPane />);

      const input = screen.getByLabelText("Max iterations");
      await user.clear(input);
      await user.type(input, "250");
      // Tabbing away is the natural "I'm done editing" gesture; no
      // Apply button needed.
      await user.tab();

      expect(extendMaxIterations).toHaveBeenCalledWith(250);
    });

    it("commits on Enter without leaving the field", async () => {
      const user = userEvent.setup();
      resetStore(baseSession);
      const extendMaxIterations = vi.fn().mockResolvedValue(undefined);
      useStore.setState({ extendMaxIterations });

      render(<RunPane />);

      const input = screen.getByLabelText("Max iterations");
      await user.clear(input);
      await user.type(input, "300{Enter}");

      expect(extendMaxIterations).toHaveBeenCalledWith(300);
    });

    it("does nothing when the typed value equals the current max", async () => {
      const user = userEvent.setup();
      resetStore({ ...baseSession, maxIterations: 100 });
      const extendMaxIterations = vi.fn().mockResolvedValue(undefined);
      useStore.setState({ extendMaxIterations });

      render(<RunPane />);

      const input = screen.getByLabelText("Max iterations");
      await user.clear(input);
      await user.type(input, "100");
      await user.tab();

      expect(extendMaxIterations).not.toHaveBeenCalled();
    });

    it("snaps the input back when the typed value is below the current iteration", async () => {
      const user = userEvent.setup();
      // Backend rejects new_total < iteration; rather than firing the
      // request and surfacing the error, the input snaps back so the
      // user sees the rejection immediately.
      resetStore({
        ...baseSession,
        mode: "paused",
        iteration: 50,
        maxIterations: 100,
      });
      const extendMaxIterations = vi.fn().mockResolvedValue(undefined);
      useStore.setState({ extendMaxIterations });

      render(<RunPane />);

      const input = screen.getByLabelText("Max iterations");
      await user.clear(input);
      await user.type(input, "25");
      await user.tab();

      expect(extendMaxIterations).not.toHaveBeenCalled();
      expect(input).toHaveValue("100");
    });

    it("is disabled while a run is actively progressing", () => {
      resetStore({ ...baseSession, mode: "playing", iteration: 5 });
      render(<RunPane />);

      expect(screen.getByLabelText("Max iterations")).toBeDisabled();
    });

    it("re-syncs the input when the session's max iterations changes externally", async () => {
      resetStore({ ...baseSession, maxIterations: 100 });
      const { rerender } = render(<RunPane />);

      // Simulate a saved-run load that pulled a different max from the file.
      resetStore({ ...baseSession, maxIterations: 500 });
      rerender(<RunPane />);

      await waitFor(() => {
        expect(screen.getByLabelText("Max iterations")).toHaveValue("500");
      });
    });
  });
});
