import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { IpcRunSeries, IpcRunStatistics } from "../../src/ipc";

vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children?: ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  LineChart: ({ data, children }: { data?: unknown; children?: ReactNode }) => (
    <div data-testid="line-chart" data-chart-data={JSON.stringify(data ?? [])}>
      {children}
    </div>
  ),
  Line: ({
    connectNulls,
    dataKey,
    name,
  }: {
    connectNulls?: boolean;
    dataKey: string;
    name?: string;
  }) => (
    <div
      data-testid="chart-line"
      data-connect-nulls={connectNulls ? "true" : "false"}
      data-key={dataKey}
      data-name={name ?? dataKey}
    />
  ),
  CartesianGrid: () => <div data-testid="cartesian-grid" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  Legend: () => <div data-testid="legend" />,
}));

vi.mock("../../src/ipc", async () => {
  const actual = await vi.importActual<typeof import("../../src/ipc")>("../../src/ipc");
  return {
    ...actual,
    readRunSeries: vi.fn(),
  };
});

import * as ipc from "../../src/ipc";
import { AggregatePane } from "../../src/panes/AggregatePane";
import { useStore, type AggregateRow } from "../../src/state/store";

const summary: IpcRunStatistics = {
  initialAliveCount: 3,
  finalAliveCount: 4,
  peakAliveCount: 8,
  peakAliveGeneration: 2,
  minAliveCount: 3,
  minAliveGeneration: 0,
  totalBirths: 9,
  totalDeaths: 5,
  iterationsRun: 3,
  status: "stable",
  cycleStartGeneration: null,
  cycleDetectedGeneration: null,
  cyclePeriod: null,
};

const seriesFor = (
  path: string,
  filename: string,
  alive = [3, 5, 8, 4],
  births = [0, 3, 4, 2],
  deaths = [0, 1, 1, 3],
): IpcRunSeries => ({
  path,
  filename,
  summary,
  series: { alive, births, deaths },
});

const readyRow = (
  path: string,
  filename: string,
  colorIndex: number,
  alive = [3, 5, 8, 4],
  births = [0, 3, 4, 2],
  deaths = [0, 1, 1, 3],
): AggregateRow => ({
  path,
  filename,
  colorIndex,
  visible: true,
  status: "ready",
  summary,
  series: { alive, births, deaths },
});

const resetStore = () => {
  useStore.setState({
    session: null,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    loadedReference: null,
    theme: "light",
    activeView: "aggregate",
    connected: false,
    initError: null,
    aggregateRows: [],
  });
  vi.clearAllMocks();
};

beforeEach(resetStore);
afterEach(() => {
  cleanup();
  resetStore();
});

describe("AggregatePane", () => {
  it("renders the empty state and Add files button", () => {
    render(<AggregatePane />);

    expect(screen.getByText("No files selected")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add files" })).toBeInTheDocument();
  });

  it("renders two ready runs as overlay series and summary rows", () => {
    useStore.setState({
      aggregateRows: [
        readyRow("C:\\runs\\run-a.gol", "run-a.gol", 0),
        readyRow("C:\\runs\\run-b.gol", "run-b.gol", 1, [2, 3, 5]),
      ],
    });

    render(<AggregatePane />);

    const aliveChart = screen.getByLabelText("Alive over time chart");
    expect(within(aliveChart).getAllByTestId("chart-line")).toHaveLength(2);
    expect(within(aliveChart).getAllByTestId("chart-line")[0]).toHaveAttribute(
      "data-connect-nulls",
      "true",
    );

    const table = screen.getByRole("table", { name: "Aggregate run summary table" });
    expect(within(table).getByText("run-a.gol")).toBeInTheDocument();
    expect(within(table).getByText("run-b.gol")).toBeInTheDocument();
  });

  it("flags summary-only files, excludes them from overlay, and includes them in the table", () => {
    useStore.setState({
      aggregateRows: [
        readyRow("C:\\runs\\ready.gol", "ready.gol", 0),
        {
          path: "C:\\runs\\legacy.gol",
          filename: "legacy.gol",
          colorIndex: 1,
          visible: true,
          status: "summaryOnly",
          summary,
        },
      ],
    });

    render(<AggregatePane />);

    expect(
      screen.getByText("summary-only — re-run to capture per-generation data"),
    ).toBeInTheDocument();
    const aliveChart = screen.getByLabelText("Alive over time chart");
    expect(within(aliveChart).getAllByTestId("chart-line")).toHaveLength(1);

    const table = screen.getByRole("table", { name: "Aggregate run summary table" });
    expect(within(table).getByText("legacy.gol")).toBeInTheDocument();
  });

  it("shows error rows and keeps them removable", async () => {
    const user = userEvent.setup();
    useStore.setState({
      aggregateRows: [
        {
          path: "C:\\runs\\bad.gol",
          filename: "bad.gol",
          colorIndex: 0,
          visible: false,
          status: "error",
          error: "content hash mismatch",
        },
      ],
    });

    render(<AggregatePane />);

    expect(screen.getByText("content hash mismatch")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Remove bad.gol" }));

    expect(screen.queryByText("bad.gol")).not.toBeInTheDocument();
  });

  it("does not add duplicate paths", async () => {
    vi.mocked(ipc.readRunSeries).mockResolvedValue(
      seriesFor("/tmp/a.gol", "a.gol"),
    );

    await useStore.getState().addAggregateFiles(["/tmp/a.gol", "/tmp/a.gol"]);
    await useStore.getState().addAggregateFiles(["/tmp/a.gol"]);

    expect(useStore.getState().aggregateRows).toHaveLength(1);
    expect(ipc.readRunSeries).toHaveBeenCalledTimes(1);
  });

  it("adds the ninth file unchecked by default", async () => {
    const visibleRows = Array.from({ length: 8 }, (_, index) =>
      readyRow(`C:\\runs\\${index}.gol`, `${index}.gol`, index),
    );
    useStore.setState({ aggregateRows: visibleRows });
    vi.mocked(ipc.readRunSeries).mockResolvedValueOnce(
      seriesFor("C:\\runs\\9.gol", "9.gol"),
    );

    await useStore.getState().addAggregateFiles(["C:\\runs\\9.gol"]);

    const ninth = useStore
      .getState()
      .aggregateRows.find((row) => row.path === "C:\\runs\\9.gol");
    expect(ninth?.visible).toBe(false);
  });

  it("updates the births/deaths chart when another run is selected", async () => {
    const user = userEvent.setup();
    useStore.setState({
      aggregateRows: [
        readyRow("C:\\runs\\first.gol", "first.gol", 0, [1, 2], [0, 1], [0, 0]),
        readyRow("C:\\runs\\second.gol", "second.gol", 1, [5, 7], [0, 9], [0, 4]),
      ],
    });

    render(<AggregatePane />);

    await waitFor(() => {
      const chart = within(screen.getByLabelText("Births and deaths chart"))
        .getByTestId("line-chart");
      expect(chart.getAttribute("data-chart-data")).toContain('"births":1');
    });

    await user.click(
      screen.getByRole("combobox", { name: "Select run for births and deaths" }),
    );
    await user.click(await screen.findByRole("option", { name: "second.gol" }));

    await waitFor(() => {
      const chart = within(screen.getByLabelText("Births and deaths chart"))
        .getByTestId("line-chart");
      expect(chart.getAttribute("data-chart-data")).toContain('"births":9');
      expect(chart.getAttribute("data-chart-data")).toContain('"deaths":4');
    });
  });
});
