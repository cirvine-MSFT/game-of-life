import { describe, expect, it } from "vitest";

import type { IpcRunStatistics, SessionInfo } from "../../src/ipc";
import {
  cycleInfoFromStats,
  formatTerminalStatus,
  formatTerminalStatusFromSession,
  formatTerminalStatusFromStats,
} from "../../src/state/terminalStatus";

const baseStats: IpcRunStatistics = {
  initialAliveCount: 3,
  finalAliveCount: 3,
  peakAliveCount: 3,
  peakAliveGeneration: 0,
  minAliveCount: 3,
  minAliveGeneration: 0,
  totalBirths: 0,
  totalDeaths: 0,
  iterationsRun: 2,
  status: "cyclic",
  cycleStartGeneration: 0,
  cycleDetectedGeneration: 2,
  cyclePeriod: 2,
};

const baseSession: SessionInfo = {
  mode: "paused",
  iteration: 2,
  width: 5,
  height: 5,
  maxIterations: 10,
  savePath: null,
  dirty: false,
  completed: true,
  jumpTarget: null,
  status: "cyclic",
};

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
    expect(desc.description).toMatch(/first seen at generation 37/);
  });

  it("falls back gracefully when cycle metadata is missing", () => {
    const desc = formatTerminalStatus("cyclic", 40);
    expect(desc.label).toBe("Cyclic at gen 40");
    expect(desc.description).not.toMatch(/period \d/);
  });

  it("handles generation zero (initial still-life)", () => {
    expect(formatTerminalStatus("stable", 0).label).toBe("Stable at gen 0");
  });

  it("extracts cycle metadata from final stats", () => {
    expect(cycleInfoFromStats(baseStats)).toEqual({
      period: 2,
      startGeneration: 0,
    });
    expect(cycleInfoFromStats(null)).toBeNull();
  });

  it("formats final stats directly", () => {
    expect(formatTerminalStatusFromStats(baseStats).label).toBe("Cyclic at gen 2 (period 2)");
  });

  it("formats a completed session with final stats", () => {
    expect(formatTerminalStatusFromSession(baseSession, baseStats)?.label).toBe(
      "Cyclic at gen 2 (period 2)",
    );
  });

  it("falls back to session iteration when final stats are not loaded yet", () => {
    expect(formatTerminalStatusFromSession(baseSession, null)?.label).toBe("Cyclic at gen 2");
  });

  it("does not format an incomplete session", () => {
    expect(
      formatTerminalStatusFromSession(
        {
          ...baseSession,
          completed: false,
          status: null,
        },
        null,
      ),
    ).toBeNull();
  });
});
