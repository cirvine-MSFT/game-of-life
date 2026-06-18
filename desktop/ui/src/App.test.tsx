import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useStore } from "./state/store";

const resetStore = () => {
  useStore.setState({
    session: null,
    board: null,
    history: [],
    latestTick: null,
    jumpProgress: null,
    finalStats: null,
    theme: "light",
    connected: false,
    initError: null,
  });
};

beforeEach(() => {
  resetStore();
});

describe("App", () => {
  it("renders the shell layout without crashing", () => {
    const { container } = render(<App />);
    // Toolbar appears once the session info loads; even before that,
    // FluentProvider must mount cleanly with no thrown errors.
    expect(container).toBeInTheDocument();
  });

  it("renders a tools panel with clear navigation affordances", () => {
    render(<App />);

    expect(screen.getByRole("complementary", { name: "Tools panel" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close tools panel" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Statistics/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Files/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Copilot/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Settings/i })).toBeInTheDocument();
    expect(screen.queryByLabelText("Collapse stats panel")).not.toBeInTheDocument();
  });

  it("collapses to a stable tools rail with a non-playback trigger", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Close tools panel" }));

    expect(screen.getByRole("button", { name: "Open tools panel" })).toBeInTheDocument();
    expect(screen.getByText("Tools")).toBeInTheDocument();
  });

  it("exposes theme selection in the settings tab", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("tab", { name: /Settings/i }));
    await user.click(screen.getByRole("radio", { name: "Dark" }));

    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    expect(useStore.getState().theme).toBe("dark");
  });
});
