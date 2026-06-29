import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { PlaybackControls } from "../../src/components/PlaybackControls";
import { useStore } from "../../src/state/store";
import type { SessionInfo } from "../../src/ipc";

// PlaybackControls lives inside the Run pane in the revamped UI. The
// "Edit Board" button is the only handoff back to the Edit destination
// — so it has to do two things: cancel any in-flight playback (return
// the session to setup) and switch activeView so the user actually
// lands in the place designed for editing.

const sessionFor = (overrides: Partial<SessionInfo>): SessionInfo => ({
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
  ...overrides,
});

const pausedSession: SessionInfo = sessionFor({ mode: "paused", iteration: 12 });

const resetStore = () => {
  useStore.setState({
    session: null,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    theme: "light",
    activeView: "run",
    connected: false,
    initError: null,
  });
};

beforeEach(resetStore);
afterEach(resetStore);

describe("PlaybackControls Edit Board handoff", () => {
  it("calls editBoard() and switches activeView to edit", async () => {
    const user = userEvent.setup();
    useStore.setState({ session: pausedSession, activeView: "run" });
    const editBoard = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ editBoard });

    render(<PlaybackControls />);

    await user.click(
      screen.getByRole("button", {
        name: /Return the board to setup and open the Edit pane/i,
      }),
    );

    // editBoard fires first so the backend cancels playing/jumping and
    // resets to setup; then setActiveView swaps the visible pane.
    expect(editBoard).toHaveBeenCalledTimes(1);
    expect(useStore.getState().activeView).toBe("edit");
  });

  it("disables Edit Board while a run is playing", () => {
    useStore.setState({ session: sessionFor({ mode: "playing", iteration: 3 }) });
    render(<PlaybackControls />);

    // Tooltip becomes the accessible name. While playing, the Edit Board
    // tooltip flips to the disabled-state copy so screen-reader users
    // hear why the button is greyed out.
    expect(
      screen.getByRole("button", {
        name: /Pause the run first, then you can edit the board/i,
      }),
    ).toBeDisabled();
  });
});

describe("PlaybackControls Play/Pause", () => {
  it("shows a single Play button in setup mode that starts the run and immediately plays", async () => {
    const user = userEvent.setup();
    useStore.setState({ session: sessionFor({ mode: "setup" }) });
    const startRun = vi.fn().mockResolvedValue(undefined);
    const play = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ startRun, play });

    render(<PlaybackControls />);

    await user.click(
      screen.getByRole("button", { name: /Start the simulation/i }),
    );

    // One click runs both: start_run leaves the backend in paused mode
    // with the initial board ready, then play kicks off the actual
    // generation loop. Two separate clicks would force the user through
    // a visible Paused step, which contradicts the "Play means play"
    // mental model.
    expect(startRun).toHaveBeenCalledTimes(1);
    expect(play).toHaveBeenCalledTimes(1);
    expect(startRun.mock.invocationCallOrder[0]).toBeLessThan(
      play.mock.invocationCallOrder[0],
    );
  });

  it("uses Play to resume from paused", async () => {
    const user = userEvent.setup();
    useStore.setState({ session: pausedSession });
    const startRun = vi.fn().mockResolvedValue(undefined);
    const play = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ startRun, play });

    render(<PlaybackControls />);

    await user.click(screen.getByRole("button", { name: /^Play \(Space\)/i }));

    expect(play).toHaveBeenCalledTimes(1);
    expect(startRun).not.toHaveBeenCalled();
  });

  it("flips Play into Pause while playing", async () => {
    const user = userEvent.setup();
    useStore.setState({
      session: sessionFor({ mode: "playing", iteration: 4 }),
    });
    const pause = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ pause });

    render(<PlaybackControls />);

    await user.click(screen.getByRole("button", { name: /Pause \(Space\)/i }));

    expect(pause).toHaveBeenCalledTimes(1);
  });

  it("does not call play(gps) until startRun() has resolved", async () => {
    const user = userEvent.setup();
    useStore.setState({ session: sessionFor({ mode: "setup" }) });

    // Make startRun hang until we explicitly resolve it so we can observe
    // the in-flight state. play must not fire while the chain is waiting.
    let resolveStart!: () => void;
    const startRun = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveStart = resolve;
        }),
    );
    const play = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ startRun, play });

    render(<PlaybackControls />);

    await user.click(
      screen.getByRole("button", { name: /Start the simulation/i }),
    );

    expect(startRun).toHaveBeenCalledTimes(1);
    expect(play).not.toHaveBeenCalled();

    resolveStart();
    // Drain the chained microtasks so the awaited play() fires.
    await Promise.resolve();
    await Promise.resolve();

    expect(play).toHaveBeenCalledTimes(1);
  });

  it("ignores a rapid second click on Play while startRun is in flight", async () => {
    const user = userEvent.setup();
    useStore.setState({ session: sessionFor({ mode: "setup" }) });

    let resolveStart!: () => void;
    const startRun = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveStart = resolve;
        }),
    );
    const play = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ startRun, play });

    render(<PlaybackControls />);

    const button = screen.getByRole("button", { name: /Start the simulation/i });
    await user.click(button);
    // Second click while the first is in flight — the button is disabled
    // by the in-flight guard, so userEvent's click is a no-op here.
    await user.click(button);

    expect(startRun).toHaveBeenCalledTimes(1);

    resolveStart();
    await Promise.resolve();
    await Promise.resolve();

    expect(play).toHaveBeenCalledTimes(1);
  });
});

describe("PlaybackControls live speed slider", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("debounces pause+play when gps changes while a run is playing", async () => {
    useStore.setState({
      session: sessionFor({ mode: "playing", iteration: 4 }),
    });
    const play = vi.fn().mockResolvedValue(undefined);
    const pause = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ play, pause });

    const { rerender } = render(<PlaybackControls />);

    // Initial render triggers the effect; the debounce timer is queued
    // but hasn't fired yet, so neither pause nor play has been called.
    expect(pause).not.toHaveBeenCalled();
    expect(play).not.toHaveBeenCalled();

    // Simulate dragging the slider through several values in quick
    // succession — only the last value should win once the timer fires.
    useStore.setState({}); // no-op set to keep zustand happy across rerenders
    rerender(<PlaybackControls />);

    // Advance time past the 200ms debounce window.
    await vi.advanceTimersByTimeAsync(250);
    // Drain awaited microtasks (pause → play).
    await Promise.resolve();
    await Promise.resolve();

    expect(pause).toHaveBeenCalledTimes(1);
    expect(play).toHaveBeenCalledTimes(1);
  });

  it("does not restart playback when gps changes while paused", async () => {
    useStore.setState({ session: pausedSession });
    const play = vi.fn().mockResolvedValue(undefined);
    const pause = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ play, pause });

    render(<PlaybackControls />);

    await vi.advanceTimersByTimeAsync(500);

    expect(pause).not.toHaveBeenCalled();
    expect(play).not.toHaveBeenCalled();
  });
});
