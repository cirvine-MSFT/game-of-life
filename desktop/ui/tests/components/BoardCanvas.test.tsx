import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SessionInfo } from "../../src/ipc";
import { useStore } from "../../src/state/store";
import { lightPalette } from "../../src/theme";
import { BoardCanvas } from "../../src/components/BoardCanvas";

const defaultResizeObserver = globalThis.ResizeObserver;

interface FillCall {
  style: string;
  rect: [number, number, number, number];
}

const baseSession: SessionInfo = {
  mode: "paused",
  iteration: 0,
  width: 3,
  height: 3,
  maxIterations: 100,
  savePath: null,
  dirty: false,
  completed: false,
  jumpTarget: null,
  status: null,
};

const verticalBlinker = new Uint8Array([
  0, 1, 0,
  0, 1, 0,
  0, 1, 0,
]);

const horizontalBlinker = new Uint8Array([
  0, 0, 0,
  1, 1, 1,
  0, 0, 0,
]);

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

const setBoard = (iteration: number, cells: Uint8Array) => {
  useStore.setState({
    session: { ...baseSession, iteration },
    board: {
      width: 3,
      height: 3,
      iteration,
      cells,
    },
  });
};

const installCanvasMock = () => {
  const fills: FillCall[] = [];
  let fillStyle = "";
  let strokeStyle = "";
  const context = {
    setTransform: vi.fn(),
    fillRect: vi.fn((x: number, y: number, width: number, height: number) => {
      fills.push({ style: fillStyle, rect: [x, y, width, height] });
    }),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    lineWidth: 1,
    set fillStyle(value: string | CanvasGradient | CanvasPattern) {
      fillStyle = String(value);
    },
    get fillStyle() {
      return fillStyle;
    },
    set strokeStyle(value: string | CanvasGradient | CanvasPattern) {
      strokeStyle = String(value);
    },
    get strokeStyle() {
      return strokeStyle;
    },
  } as unknown as CanvasRenderingContext2D;

  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(() => context);
  return fills;
};

const installImmediateResizeObserver = () => {
  let emitResize: () => void = () => undefined;

  class ImmediateResizeObserver {
    callback: ResizeObserverCallback;

    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
      emitResize = () => this.callback([], this as unknown as ResizeObserver);
    }

    observe() {
      emitResize();
    }

    unobserve() {}

    disconnect() {}
  }

  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    writable: true,
    value: ImmediateResizeObserver,
  });

  return emitResize;
};

beforeEach(() => {
  resetStore();
  vi.clearAllMocks();
  Object.defineProperty(HTMLElement.prototype, "clientWidth", {
    configurable: true,
    get: () => 90,
  });
  Object.defineProperty(HTMLElement.prototype, "clientHeight", {
    configurable: true,
    get: () => 90,
  });
  Object.defineProperty(window, "requestAnimationFrame", {
    configurable: true,
    writable: true,
    value: vi.fn(() => 1),
  });
  Object.defineProperty(window, "cancelAnimationFrame", {
    configurable: true,
    writable: true,
    value: vi.fn(),
  });
});

afterEach(() => {
  cleanup();
  resetStore();
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    writable: true,
    value: defaultResizeObserver,
  });
  vi.restoreAllMocks();
});

describe("BoardCanvas transition animation", () => {
  it("highlights births and deaths for a sequential generation tick", () => {
    const fills = installCanvasMock();
    setBoard(0, verticalBlinker);

    render(<BoardCanvas paletteName="light" />);
    fills.length = 0;

    act(() => {
      useStore.setState({
        session: { ...baseSession, iteration: 1 },
        board: { width: 3, height: 3, iteration: 1, cells: horizontalBlinker },
        latestTick: { iteration: 1, alive: 3, dead: 6, births: 2, deaths: 2 },
      });
    });

    expect(fills.some((call) => call.style === lightPalette.resurrecting)).toBe(true);
    expect(fills.some((call) => call.style === lightPalette.dying)).toBe(true);
  });

  it("does not animate when the incoming board skips generations", () => {
    const fills = installCanvasMock();
    setBoard(0, verticalBlinker);

    render(<BoardCanvas paletteName="light" />);
    fills.length = 0;

    act(() => {
      useStore.setState({
        session: { ...baseSession, iteration: 3 },
        board: { width: 3, height: 3, iteration: 3, cells: horizontalBlinker },
        latestTick: { iteration: 3, alive: 3, dead: 6, births: 2, deaths: 2 },
      });
    });

    expect(fills.some((call) => call.style === lightPalette.resurrecting)).toBe(false);
    expect(fills.some((call) => call.style === lightPalette.dying)).toBe(false);
  });

  it("does not cancel transition animation for ResizeObserver initial callbacks", () => {
    const emitResize = installImmediateResizeObserver();
    installCanvasMock();
    setBoard(0, verticalBlinker);

    render(<BoardCanvas paletteName="light" />);

    act(() => {
      useStore.setState({
        session: { ...baseSession, iteration: 1 },
        board: { width: 3, height: 3, iteration: 1, cells: horizontalBlinker },
        latestTick: { iteration: 1, alive: 3, dead: 6, births: 2, deaths: 2 },
      });
    });

    emitResize();

    expect(window.cancelAnimationFrame).not.toHaveBeenCalled();
  });
});
