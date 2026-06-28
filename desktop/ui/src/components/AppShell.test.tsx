import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../App";
import { useStore } from "../state/store";

const resetStore = () => {
  useStore.setState({
    session: null,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    theme: "light",
    activeView: "edit",
    connected: false,
    initError: null,
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
    expect(screen.getByLabelText("Run placeholder")).toBeInTheDocument();
    expect(useStore.getState().activeView).toBe("run");

    await user.click(
      screen.getByRole("tab", { name: "Aggregate statistics" }),
    );
    expect(
      screen.getByLabelText("Aggregate statistics placeholder"),
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
  });
});
