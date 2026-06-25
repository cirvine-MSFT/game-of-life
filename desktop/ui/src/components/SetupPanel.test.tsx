import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FluentProvider, webLightTheme } from "@fluentui/react-components";

import { SetupPanel } from "./SetupPanel";
import type { SessionInfo } from "../ipc";

vi.mock("../ipc", async () => {
  const actual = await vi.importActual<typeof import("../ipc")>("../ipc");
  return {
    ...actual,
    applyPattern: vi.fn(async () => undefined),
    randomize: vi.fn(async () => undefined),
    clearBoard: vi.fn(async () => undefined),
    getSession: vi.fn(async () => baseSession),
    getBoard: vi.fn(async () => ({
      width: 20,
      height: 20,
      iteration: 0,
      cellsBase64: "",
    })),
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

const seed = (session: SessionInfo | null = baseSession) => {
  useStore.setState({
    session,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    theme: "light",
    connected: true,
    initError: null,
    newRunDialogOpen: false,
  });
};

const renderPanel = () =>
  render(
    <FluentProvider theme={webLightTheme}>
      <SetupPanel />
    </FluentProvider>,
  );

beforeEach(() => {
  vi.clearAllMocks();
  seed();
});

describe("SetupPanel", () => {
  it("opens the New Run dialog when the user clicks New Run…", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(screen.getByRole("button", { name: /New Run/ }));

    expect(useStore.getState().newRunDialogOpen).toBe(true);
  });

  it("calls clearBoard when Clear board is clicked in setup mode", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(screen.getByRole("button", { name: "Clear board" }));

    expect(ipc.clearBoard).toHaveBeenCalledTimes(1);
  });

  it("calls applyPattern with the selected pattern", async () => {
    const user = userEvent.setup();
    renderPanel();

    // Default is demo; click Apply.
    await user.click(screen.getByRole("button", { name: "Apply" }));

    expect(ipc.applyPattern).toHaveBeenCalledWith("demo");
  });

  it("calls randomize with parsed seed and density", async () => {
    const user = userEvent.setup();
    renderPanel();

    const seedInput = screen.getByRole("spinbutton", { name: "Seed" });
    const densityInput = screen.getByRole("spinbutton", { name: "Alive per 1000" });
    await user.clear(seedInput);
    await user.type(seedInput, "42");
    await user.clear(densityInput);
    await user.type(densityInput, "250");
    await user.click(screen.getByRole("button", { name: "Randomize" }));

    expect(ipc.randomize).toHaveBeenCalledWith(42, 250);
  });

  it("disables in-place actions when not in setup mode", () => {
    seed({ ...baseSession, mode: "paused" });
    renderPanel();

    expect(screen.getByRole("button", { name: "Clear board" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Randomize" })).toBeDisabled();
    // New Run launcher stays enabled because the backend stops the
    // worker on create_run.
    expect(screen.getByRole("button", { name: /New Run/ })).toBeEnabled();
  });

  it("shows a validation error for invalid density and skips IPC", async () => {
    const user = userEvent.setup();
    renderPanel();

    const densityInput = screen.getByRole("spinbutton", { name: "Alive per 1000" });
    await user.clear(densityInput);
    await user.type(densityInput, "2000");
    await user.click(screen.getByRole("button", { name: "Randomize" }));

    expect(ipc.randomize).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(/density/i);
  });
});
