import { describe, it, expect, beforeEach } from "vitest";

import {
  decodeBoard,
  type BoardPayload,
  type IpcRunSeries,
  type SessionInfo,
} from "../../src/ipc/types";

describe("decodeBoard", () => {
  it("round-trips a small board through base64", () => {
    const cells = new Uint8Array([0, 1, 1, 0, 1, 0, 0, 1, 1]);
    const cellsBase64 = btoa(String.fromCharCode(...cells));
    const payload: BoardPayload = {
      width: 3,
      height: 3,
      iteration: 5,
      cellsBase64,
    };
    const decoded = decodeBoard(payload);
    expect(decoded.width).toBe(3);
    expect(decoded.height).toBe(3);
    expect(decoded.iteration).toBe(5);
    expect(Array.from(decoded.cells)).toEqual(Array.from(cells));
  });

  it("handles an empty board", () => {
    const payload: BoardPayload = {
      width: 0,
      height: 0,
      iteration: 0,
      cellsBase64: "",
    };
    const decoded = decodeBoard(payload);
    expect(decoded.cells.length).toBe(0);
  });
});

describe("SessionInfo shape", () => {
  // Type-level: exercising the union to make sure changes to the Rust
  // types are mirrored here. If a field is renamed in `ipc_types.rs`,
  // TypeScript will fail this assignment at compile time.
  beforeEach(() => undefined);

  it("accepts a fully populated session", () => {
    const info: SessionInfo = {
      mode: "playing",
      iteration: 42,
      width: 50,
      height: 50,
      maxIterations: 100,
      savePath: "/tmp/run.gol",
      dirty: true,
      completed: false,
      jumpTarget: 80,
      status: null,
    };
    expect(info.mode).toBe("playing");
  });

  it("accepts a completed extinct session", () => {
    const info: SessionInfo = {
      mode: "paused",
      iteration: 17,
      width: 10,
      height: 10,
      maxIterations: 100,
      savePath: null,
      dirty: false,
      completed: true,
      jumpTarget: null,
      status: "extinct",
    };
    expect(info.status).toBe("extinct");
  });

  it("accepts a completed stable session", () => {
    const info: SessionInfo = {
      mode: "paused",
      iteration: 1,
      width: 2,
      height: 2,
      maxIterations: 10,
      savePath: null,
      dirty: false,
      completed: true,
      jumpTarget: null,
      status: "stable",
    };
    expect(info.status).toBe("stable");
  });

  it("accepts a completed cyclic session", () => {
    const info: SessionInfo = {
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
    expect(info.status).toBe("cyclic");
  });
});

describe("IpcRunSeries shape", () => {
  it("accepts a saved run with per-iteration series", () => {
    const loaded: IpcRunSeries = {
      path: "C:\\runs\\blinker.gol",
      filename: "blinker.gol",
      summary: {
        initialAliveCount: 3,
        finalAliveCount: 3,
        peakAliveCount: 3,
        peakAliveGeneration: 0,
        minAliveCount: 3,
        minAliveGeneration: 0,
        totalBirths: 4,
        totalDeaths: 4,
        iterationsRun: 2,
        status: "maxIterations",
        cycleStartGeneration: null,
        cycleDetectedGeneration: null,
        cyclePeriod: null,
      },
      series: {
        alive: [3, 3, 3],
        births: [0, 2, 2],
        deaths: [0, 2, 2],
      },
    };

    expect(loaded.series?.alive).toEqual([3, 3, 3]);
  });
});
