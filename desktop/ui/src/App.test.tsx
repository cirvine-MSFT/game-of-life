import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { App } from "./App";

describe("App", () => {
  it("renders the shell layout without crashing", () => {
    const { container } = render(<App />);
    // Toolbar appears once the session info loads; even before that,
    // FluentProvider must mount cleanly with no thrown errors.
    expect(container).toBeInTheDocument();
  });
});
