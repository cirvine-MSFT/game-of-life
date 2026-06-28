import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { ChatPanel } from "./ChatPanel";
import { AiNotImplementedError } from "./errors";

describe("ChatPanel", () => {
  it("shows the empty hint until the user sends a prompt", () => {
    render(
      <ChatPanel
        provider={() => Promise.resolve("ok")}
        emptyHint="No messages yet."
        ariaLabel="Test chat"
      />,
    );
    expect(screen.getByText("No messages yet.")).toBeInTheDocument();
  });

  it("renders an error bubble when the provider rejects, and keeps the input enabled", async () => {
    const user = userEvent.setup();
    render(
      <ChatPanel
        provider={() => Promise.reject(new AiNotImplementedError())}
        emptyHint="empty"
        ariaLabel="Test chat"
      />,
    );

    const input = screen.getByLabelText("Test chat prompt");
    await user.type(input, "hello");
    await user.click(screen.getByRole("button", { name: "Send" }));

    const errorBubble = await screen.findByText(
      /AI integration is not wired up yet/i,
    );
    expect(errorBubble).toBeInTheDocument();
    expect(errorBubble.closest("[data-role]")?.getAttribute("data-role")).toBe(
      "error",
    );
    expect(input).not.toBeDisabled();
  });

  it("appends the assistant reply on success", async () => {
    const user = userEvent.setup();
    render(
      <ChatPanel
        provider={(prompt) => Promise.resolve(`echo: ${prompt}`)}
        emptyHint="empty"
        ariaLabel="Test chat"
      />,
    );

    const input = screen.getByLabelText("Test chat prompt");
    await user.type(input, "ping");
    await user.click(screen.getByRole("button", { name: "Send" }));

    expect(await screen.findByText("ping")).toBeInTheDocument();
    expect(await screen.findByText("echo: ping")).toBeInTheDocument();
  });

  it("ignores empty prompts", async () => {
    const user = userEvent.setup();
    const provider = (): Promise<string> => Promise.resolve("never");
    render(
      <ChatPanel
        provider={provider}
        emptyHint="empty hint"
        ariaLabel="Test chat"
      />,
    );

    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
    await user.type(screen.getByLabelText("Test chat prompt"), "   ");
    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
    expect(screen.getByText("empty hint")).toBeInTheDocument();
  });
});
