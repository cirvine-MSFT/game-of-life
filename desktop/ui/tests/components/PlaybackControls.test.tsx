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

const pausedSession: SessionInfo = {
  mode: "paused",
  iteration: 12,
  width: 20,
  height: 20,
  maxIterations: 100,
  savePath: null,
  dirty: false,
  completed: false,
  jumpTarget: null,
  status: null,
};

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
});
