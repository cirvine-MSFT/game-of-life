import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useStore } from "./state/store";
import type { IpcRunStatistics, IpcRunStatus, SessionInfo } from "./ipc";

const resetStore = () => {
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

const completedSession = (status: IpcRunStatus, iteration: number): SessionInfo => ({
  mode: "paused",
  iteration,
  width: 5,
  height: 5,
  maxIterations: 100,
  savePath: null,
  dirty: false,
  completed: true,
  jumpTarget: null,
  status,
});

const completedStats = (
  status: IpcRunStatus,
  iterationsRun: number,
  cycle?: { period: number; start: number; detected: number },
): IpcRunStatistics => ({
  initialAliveCount: 0,
  finalAliveCount: 0,
  peakAliveCount: 0,
  peakAliveGeneration: 0,
  minAliveCount: 0,
  minAliveGeneration: 0,
  totalBirths: 0,
  totalDeaths: 0,
  iterationsRun,
  status,
  cycleStartGeneration: cycle?.start ?? null,
  cycleDetectedGeneration: cycle?.detected ?? null,
  cyclePeriod: cycle?.period ?? null,
});

beforeEach(() => {
  resetStore();
});

describe("App", () => {
  it("renders the shell layout without crashing", () => {
    const { container } = render(<App />);
    // Toolbar appears once the session info loads; even before that,
    // FluentProvider must mount cleanly with no thrown errors.
    expect(container).toBeInTheDocument();
  });

  it("renders a tools panel with clear navigation affordances", () => {
    render(<App />);

    expect(screen.getByRole("complementary", { name: "Tools panel" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close tools panel" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Statistics/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Files/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Copilot/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Settings/i })).toBeInTheDocument();
    expect(screen.queryByLabelText("Collapse stats panel")).not.toBeInTheDocument();
  });

  it("keeps file actions out of the primary playback toolbar", () => {
    render(<App />);

    expect(screen.queryByRole("button", { name: /Load board/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Save board/i })).not.toBeInTheDocument();
  });

  it("collapses to a stable tools rail with a non-playback trigger", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Close tools panel" }));

    expect(screen.getByRole("button", { name: "Open tools panel" })).toBeInTheDocument();
    expect(screen.getByText("Tools")).toBeInTheDocument();
  });

  it("exposes theme selection in the settings tab", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("tab", { name: /Settings/i }));
    await user.click(screen.getByRole("radio", { name: "Dark" }));

    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    expect(useStore.getState().theme).toBe("dark");
  });

  it("shows load and save workflow details in the files tab", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("tab", { name: /Files/i }));

    expect(screen.getByRole("region", { name: "Files panel" })).toBeInTheDocument();
    expect(screen.getByText(/replayed or adjusted/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Load board snapshot" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Load run initial" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Load run final" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save board snapshot" })).toBeInTheDocument();
    expect(screen.getByLabelText("Current board file status")).toBeInTheDocument();
  });

  describe("terminal-state UX", () => {
    const seedCompletedSession = (
      session: SessionInfo,
      stats: IpcRunStatistics,
    ) => {
      useStore.setState({
        connected: true,
        session,
        finalStats: stats,
      });
    };

    it("surfaces a stable outcome on the toolbar, status bar, and stats panel", async () => {
      const user = userEvent.setup();
      seedCompletedSession(completedSession("stable", 12), completedStats("stable", 12));
      render(<App />);

      expect((await screen.findAllByText("Stable")).length).toBeGreaterThan(0);
      expect(screen.getByText(/Stable at gen 12/)).toBeInTheDocument();

      await user.click(screen.getByRole("tab", { name: /Statistics/i }));
      expect(screen.getByText("Stopped at generation")).toBeInTheDocument();
    });

    it("surfaces a cyclic outcome with period", async () => {
      seedCompletedSession(
        completedSession("cyclic", 40),
        completedStats("cyclic", 40, { period: 3, start: 37, detected: 40 }),
      );
      render(<App />);

      expect((await screen.findAllByText("Cyclic")).length).toBeGreaterThan(0);
      expect(screen.getByText(/Cyclic at gen 40 \(period 3\)/)).toBeInTheDocument();
    });

    it("surfaces an extinct outcome", async () => {
      seedCompletedSession(completedSession("extinct", 7), completedStats("extinct", 7));
      render(<App />);

      expect((await screen.findAllByText("Extinct")).length).toBeGreaterThan(0);
      expect(screen.getByText(/Extinct at gen 7/)).toBeInTheDocument();
    });

    it("surfaces a max-iterations outcome", async () => {
      seedCompletedSession(
        completedSession("maxIterations", 100),
        completedStats("maxIterations", 100),
      );
      render(<App />);

      expect((await screen.findAllByText("Reached max")).length).toBeGreaterThan(0);
      expect(screen.getByText(/Reached max \(100\)/)).toBeInTheDocument();
    });
  });
});
