import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SettingsPane } from "../../src/panes/SettingsPane";
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
    activeView: "settings",
    animateTransitions: true,
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

describe("SettingsPane", () => {
  it("renders the theme radio group bound to the store", async () => {
    const user = userEvent.setup();
    render(<SettingsPane />);

    expect(screen.getByRole("radio", { name: "Light" })).toBeChecked();
    await user.click(screen.getByRole("radio", { name: "Dark" }));
    expect(useStore.getState().theme).toBe("dark");
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
  });

  it("renders the AI theme chat stub and surfaces its rejection as an error bubble", async () => {
    const user = userEvent.setup();
    render(<SettingsPane />);

    const input = screen.getByLabelText("Theme tweak chat prompt");
    await user.type(input, "Make it red");
    await user.click(screen.getByRole("button", { name: "Send" }));

    const errorBubble = await screen.findByText(
      /AI integration is not wired up yet/i,
    );
    expect(errorBubble.closest("[data-role]")?.getAttribute("data-role")).toBe(
      "error",
    );
  });

  it("renders a cell-transition toggle bound to the store and persists changes", async () => {
    const user = userEvent.setup();
    render(<SettingsPane />);

    const toggle = screen.getByRole("switch", {
      name: /Animate cell births and deaths/i,
    });
    expect(toggle).toBeChecked();

    await user.click(toggle);

    expect(useStore.getState().animateTransitions).toBe(false);
    expect(localStorage.getItem("gol.animateTransitions")).toBe("false");
    expect(
      screen.getByRole("switch", {
        name: /Animations off — instant cell updates/i,
      }),
    ).not.toBeChecked();
  });
});
