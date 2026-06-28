import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";

import { App } from "./App";
import { useStore } from "./state/store";

// The detailed pane-by-pane assertions that used to live in this file
// (covering the old right-side ToolsPanel, Files tab, terminal-state
// badges, etc.) have moved to dedicated per-pane test files alongside
// the new AppShell. This file now just smoke-tests the top-level mount
// path and the connection-error retry surface.

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
};

beforeEach(resetStore);

describe("App", () => {
  it("renders the AppShell with the nav rail visible", () => {
    render(<App />);
    expect(
      screen.getByRole("navigation", { name: "Primary navigation" }),
    ).toBeInTheDocument();
  });

  it("renders the connection-error UI when initError is set", () => {
    // Mark the store as already connected so AppShell's mount-time
    // useEffect short-circuits in `connect()` and leaves initError alone.
    useStore.setState({ initError: "backend offline", connected: true });
    render(<App />);
    expect(
      screen.getByText("Failed to connect to the simulation backend."),
    ).toBeInTheDocument();
    expect(screen.getByText("backend offline")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
  });
});
