import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// jsdom doesn't ship ResizeObserver, which BoardCanvas and Recharts
// both use. A no-op stub is enough for smoke-rendering tests; the
// canvas just won't redraw on resize.
if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
}

// jsdom exposes canvas elements but not a 2D implementation. Components
// only need a harmless context for smoke rendering; focused canvas tests
// replace this stub with call-recording mocks.
HTMLCanvasElement.prototype.getContext = vi.fn(
  () =>
    ({
      setTransform: vi.fn(),
      fillRect: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      stroke: vi.fn(),
      fillStyle: "",
      strokeStyle: "",
      lineWidth: 1,
    }) as unknown as CanvasRenderingContext2D,
) as unknown as typeof HTMLCanvasElement.prototype.getContext;

// Stub out Tauri's IPC bridge so component tests can render without a
// real Tauri runtime. Each command returns a sensible default; tests
// that need richer behaviour override with `vi.mocked(...)`.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === "get_session") {
      return {
        mode: "setup",
        iteration: 0,
        width: 0,
        height: 0,
        maxIterations: 0,
        savePath: null,
        dirty: false,
        completed: false,
        jumpTarget: null,
        status: null,
      };
    }
    if (command === "get_board") {
      return { width: 0, height: 0, iteration: 0, cellsBase64: "" };
    }
    if (command === "get_alive_history") {
      return [];
    }
    if (command === "get_final_stats") {
      return null;
    }
    return undefined;
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => undefined),
}));
