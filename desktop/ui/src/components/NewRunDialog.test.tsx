import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FluentProvider, webLightTheme } from "@fluentui/react-components";

import { NewRunDialog } from "./NewRunDialog";
import type { SessionInfo } from "../ipc";

const dialog = vi.hoisted(() => ({
  ask: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => dialog);

vi.mock("../ipc", async () => {
  const actual = await vi.importActual<typeof import("../ipc")>("../ipc");
  return {
    ...actual,
    createRun: vi.fn(async () => undefined),
    getSession: vi.fn(),
    getBoard: vi.fn(async () => ({
      width: 0,
      height: 0,
      iteration: 0,
      cellsBase64: "",
    })),
    getAliveHistory: vi.fn(async () => []),
  };
});

import * as ipc from "../ipc";
import { useStore } from "../state/store";

const baseSession: SessionInfo = {
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

const resetStore = (overrides: Partial<{ session: SessionInfo | null }> = {}) => {
  useStore.setState({
    session: overrides.session === undefined ? baseSession : overrides.session,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    theme: "light",
    connected: true,
    initError: null,
    newRunDialogOpen: true,
  });
};

const renderDialog = () =>
  render(
    <FluentProvider theme={webLightTheme}>
      <NewRunDialog />
    </FluentProvider>,
  );

beforeEach(() => {
  vi.clearAllMocks();
  dialog.ask.mockResolvedValue(true);
  vi.mocked(ipc.getSession).mockResolvedValue(baseSession);
  resetStore();
});

afterEach(() => {
  useStore.setState({ newRunDialogOpen: false });
});

const fillNumberInput = async (
  user: ReturnType<typeof userEvent.setup>,
  label: string,
  value: string,
) => {
  const input = screen.getByRole("spinbutton", { name: label });
  await user.clear(input);
  await user.type(input, value);
};

describe("NewRunDialog", () => {
  it("submits with default values when the user clicks Create", async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(ipc.createRun).toHaveBeenCalledWith({
      width: 20,
      height: 20,
      maxIterations: 100,
      source: { kind: "pattern", value: "demo" },
    });
    // The store closes the dialog on success.
    expect(useStore.getState().newRunDialogOpen).toBe(false);
  });

  it("rejects invalid width with an inline error and skips IPC", async () => {
    const user = userEvent.setup();
    renderDialog();

    await fillNumberInput(user, "Width", "0");
    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(ipc.createRun).not.toHaveBeenCalled();
    expect(screen.getByText(/Width must be a whole number/i)).toBeInTheDocument();
  });

  it("rejects invalid max iterations with an inline error and skips IPC", async () => {
    const user = userEvent.setup();
    renderDialog();

    await fillNumberInput(user, "Max iterations", "0");
    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(ipc.createRun).not.toHaveBeenCalled();
    expect(screen.getByText(/Max iterations must be a whole number/i)).toBeInTheDocument();
  });

  it("shows seed and density inputs only when source is random", async () => {
    const user = userEvent.setup();
    renderDialog();

    expect(screen.queryByRole("spinbutton", { name: "Seed" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /Random/ }));

    expect(screen.getByRole("spinbutton", { name: "Seed" })).toBeInTheDocument();
    expect(
      screen.getByRole("spinbutton", { name: "Alive cells per 1000" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(ipc.createRun).toHaveBeenCalledWith({
      width: 20,
      height: 20,
      maxIterations: 100,
      source: {
        kind: "random",
        value: { seed: 1, aliveCellsPerThousand: 300 },
      },
    });
  });

  it("surfaces a backend error message inline and keeps the dialog open", async () => {
    vi.mocked(ipc.createRun).mockRejectedValueOnce({
      kind: "streamingNotImplemented",
      message: "Board needs more memory than the desktop budget.",
    });
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole("button", { name: "Create" }));

    const bar = await screen.findByLabelText("New Run error");
    expect(within(bar).getByText(/desktop budget/i)).toBeInTheDocument();
    expect(useStore.getState().newRunDialogOpen).toBe(true);
  });

  it("asks for confirmation before discarding a dirty board", async () => {
    dialog.ask.mockResolvedValueOnce(false);
    resetStore({ session: { ...baseSession, dirty: true } });
    useStore.setState({ newRunDialogOpen: true });
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(dialog.ask).toHaveBeenCalledTimes(1);
    expect(ipc.createRun).not.toHaveBeenCalled();
    expect(useStore.getState().newRunDialogOpen).toBe(true);
  });

  it("closes when the user clicks Cancel", async () => {
    const user = userEvent.setup();
    renderDialog();

    await act(async () => {
      await user.click(screen.getByRole("button", { name: "Cancel" }));
    });

    expect(useStore.getState().newRunDialogOpen).toBe(false);
    expect(ipc.createRun).not.toHaveBeenCalled();
  });
});
