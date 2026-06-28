import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SettingsPane } from "./SettingsPane";
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
    activeView: "settings",
    connected: false,
    initError: null,
  });
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
});
