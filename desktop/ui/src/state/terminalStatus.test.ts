import { describe, expect, it } from "vitest";

import { formatTerminalStatus } from "./terminalStatus";

describe("formatTerminalStatus", () => {
  it("describes a stable fixed-point outcome", () => {
    const desc = formatTerminalStatus("stable", 12);
    expect(desc.shortLabel).toBe("Stable");
    expect(desc.label).toBe("Stable at gen 12");
    expect(desc.color).toBe("success");
    expect(desc.description).toMatch(/fixed-point/i);
  });

  it("describes an extinct outcome", () => {
    const desc = formatTerminalStatus("extinct", 7);
    expect(desc.shortLabel).toBe("Extinct");
    expect(desc.label).toBe("Extinct at gen 7");
    expect(desc.color).toBe("severe");
    expect(desc.description).toMatch(/every cell died/i);
  });

  it("describes a max-iterations outcome", () => {
    const desc = formatTerminalStatus("maxIterations", 100);
    expect(desc.shortLabel).toBe("Reached max");
    expect(desc.label).toBe("Reached max (100)");
    expect(desc.color).toBe("warning");
    expect(desc.description).toMatch(/maximum of 100/i);
  });

  it("describes a cyclic outcome with period metadata", () => {
    const desc = formatTerminalStatus("cyclic", 40, { period: 3, startGeneration: 37 });
    expect(desc.shortLabel).toBe("Cyclic");
    expect(desc.label).toBe("Cyclic at gen 40 (period 3)");
    expect(desc.color).toBe("brand");
    expect(desc.description).toMatch(/period 3/);
  });

  it("falls back gracefully when cycle metadata is missing", () => {
    const desc = formatTerminalStatus("cyclic", 40);
    expect(desc.label).toBe("Cyclic at gen 40");
    expect(desc.description).not.toMatch(/period \d/);
  });

  it("handles generation zero (initial still-life)", () => {
    expect(formatTerminalStatus("stable", 0).label).toBe("Stable at gen 0");
  });
});
