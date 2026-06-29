import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../src/App";
import { useStore } from "../../src/state/store";

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
    activeView: "edit",
    connected: false,
    initError: null,
    aggregateRows: [],
  });
  try {
    localStorage.clear();
  } catch {
    // ignore in restricted contexts
  }
};

beforeEach(resetStore);
afterEach(resetStore);

describe("AppShell nav rail", () => {
  it("defaults to the Edit pane on first render", () => {
    render(<App />);
    expect(screen.getByRole("region", { name: "Edit board" })).toBeInTheDocument();
  });

  it("switches the active pane when a rail tab is clicked", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("tab", { name: "Run" }));
    expect(screen.getByRole("region", { name: "Run" })).toBeInTheDocument();
    expect(useStore.getState().activeView).toBe("run");

    await user.click(
      screen.getByRole("tab", { name: "Aggregate statistics" }),
    );
    expect(
      screen.getByRole("region", { name: "Aggregate statistics" }),
    ).toBeInTheDocument();
    expect(useStore.getState().activeView).toBe("aggregate");
  });

  it("renders the telemetry tab as disabled and ignores clicks", async () => {
    const user = userEvent.setup();
    render(<App />);

    const telemetry = screen.getByRole("tab", { name: "Telemetry" });
    // Fluent's Tab renders aria-disabled; the underlying button is also
    // disabled. Either is sufficient for assistive tech to skip the tab.
    expect(telemetry).toHaveAttribute("aria-disabled", "true");

    // userEvent honors disabled state and will throw — short-circuit by
    // checking that a click does not change activeView even if it goes
    // through.
    await user.click(telemetry).catch(() => undefined);
    expect(useStore.getState().activeView).toBe("edit");
  });

  it("renders a status bar with pane-appropriate text", async () => {
    const user = userEvent.setup();
    render(<App />);

    const statusBar = screen.getByLabelText("Status bar");
    expect(statusBar).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "Settings" }));
    expect(screen.getByLabelText("Status bar")).toHaveTextContent(/Theme/);

    useStore.setState({
      aggregateRows: [
        {
          path: "C:\\runs\\bad.gol",
          filename: "bad.gol",
          status: "error",
          colorIndex: 0,
          visible: false,
          error: "bad magic",
        },
      ],
    });
    await user.click(
      screen.getByRole("tab", { name: "Aggregate statistics" }),
    );
    expect(screen.getByLabelText("Status bar")).toHaveTextContent(
      "Selected: 1 · Loaded: 0 · Errors: 1",
    );
    expect(screen.getByLabelText("Status bar")).toHaveTextContent("bad magic");
  });
});
