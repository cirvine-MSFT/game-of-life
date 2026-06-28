import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";

import { EditPane } from "../../src/panes/EditPane";
import type { SessionInfo } from "../../src/ipc";
import { useStore } from "../../src/state/store";

const baseSession: SessionInfo = {
  mode: "setup",
  iteration: 0,
  width: 5,
  height: 4,
  maxIterations: 100,
  savePath: null,
  dirty: false,
  completed: false,
  jumpTarget: null,
  status: null,
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
    activeView: "edit",
    connected: false,
    initError: null,
    aggregateRows: [],
  });
  vi.mocked(invoke).mockClear();
};

beforeEach(() => resetStore());
afterEach(() => {
  resetStore();
});

describe("EditPane", () => {
  it("renders setup editing controls enabled and an editable board", () => {
    render(<EditPane />);

    expect(screen.getByRole("toolbar", { name: "Edit board toolbar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Apply size" })).not.toBeDisabled();
    expect(screen.getByRole("button", { name: "Clear" })).not.toBeDisabled();
    expect(screen.getByRole("button", { name: /Randomize/ })).not.toBeDisabled();
    expect(screen.getByRole("button", { name: "Pattern" })).not.toBeDisabled();
    expect(screen.getByRole("button", { name: "Save board" })).not.toBeDisabled();
    expect(screen.getByRole("button", { name: "Send to Run ▶" })).not.toBeDisabled();
    expect(
      screen.getByRole("img", { name: "Game of Life board 5 by 4" }),
    ).toHaveStyle({ cursor: "crosshair" });
  });

  it("switches to Run without mutating the board", async () => {
    const user = userEvent.setup();
    render(<EditPane />);

    await user.click(screen.getByRole("button", { name: "Send to Run ▶" }));

    expect(useStore.getState().activeView).toBe("run");
    const mutatingCommands = vi
      .mocked(invoke)
      .mock.calls.map(([command]) => command)
      .filter((command) =>
        [
          "set_cell",
          "paint_cells",
          "apply_pattern",
          "randomize",
          "clear_board",
          "create_run",
          "load_board_snapshot",
          "save_board_snapshot",
        ].includes(String(command)),
      );
    expect(mutatingCommands).toEqual([]);
  });

  it("disables setup-only buttons while playing", () => {
    resetStore({ ...baseSession, mode: "playing" });

    render(<EditPane />);

    expect(screen.getByRole("button", { name: "Apply size" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Clear" })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Randomize/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Pattern" })).toBeDisabled();
  });

  it("disables setup-only buttons until the session is loaded", () => {
    resetStore(null);

    render(<EditPane />);

    expect(screen.getByRole("button", { name: "Apply size" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Clear" })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Randomize/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Pattern" })).toBeDisabled();
  });

  it("shows active-run actions and wires them to navigation and edit mode", async () => {
    const user = userEvent.setup();
    resetStore({ ...baseSession, mode: "playing" });
    render(<EditPane />);

    expect(
      screen.getByText("A run is in progress. Edit the current board state, or return to setup."),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Go to Run" }));
    expect(useStore.getState().activeView).toBe("run");

    await user.click(screen.getByRole("button", { name: "Return to setup" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("edit_board");
    });
  });

  it("surfaces the board-generation AI stub rejection", async () => {
    const user = userEvent.setup();
    render(<EditPane />);

    await user.type(
      screen.getByLabelText("Board generation chat prompt"),
      "Make a glider",
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    const errorBubble = await screen.findByText(
      /AI integration is not wired up yet/i,
    );
    expect(errorBubble.closest("[data-role]")?.getAttribute("data-role")).toBe(
      "error",
    );
  });
});
