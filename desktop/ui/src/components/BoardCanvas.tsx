import { useCallback, useEffect, useRef } from "react";
import { makeStyles, tokens } from "@fluentui/react-components";

import { useStore } from "../state/store";
import { paletteFor, type CellPalette, type PaletteName } from "../theme";

const useStyles = makeStyles({
  root: {
    position: "relative",
    width: "100%",
    height: "100%",
    backgroundColor: tokens.colorNeutralBackground3,
    overflow: "hidden",
  },
  canvas: {
    display: "block",
    width: "100%",
    height: "100%",
    imageRendering: "pixelated",
  },
});

interface BoardCanvasProps {
  paletteName: PaletteName;
  readOnly?: boolean;
}

interface DragState {
  /** The alive-value the drag started with; we paint a continuous stroke
   *  of the opposite of the cell that was clicked, so a single drag
   *  feels like a toggle-and-extend rather than an unpredictable mix. */
  paintingAlive: boolean;
  lastX: number;
  lastY: number;
}

interface RenderedBoard {
  width: number;
  height: number;
  iteration: number;
  cells: Uint8Array;
}

const TRANSITION_DURATION_MS = 550;

/**
 * Renders the current board onto a Canvas 2D surface and wires pointer
 * events for click + drag-paint when the session is in Setup mode.
 */
export const BoardCanvas = ({ paletteName, readOnly = false }: BoardCanvasProps) => {
  const styles = useStyles();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const dragRef = useRef<DragState | null>(null);
  const previousRenderRef = useRef<RenderedBoard | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const renderBoardRef = useRef<(animateTransitions: boolean) => void>(() => undefined);
  const board = useStore((s) => s.board);
  const latestTick = useStore((s) => s.latestTick);
  const sessionMode = useStore((s) => s.session?.mode ?? "setup");
  const setCellAction = useStore((s) => s.setCell);

  const palette = paletteFor(paletteName);

  const cancelAnimation = useCallback(() => {
    if (animationFrameRef.current !== null) {
      window.cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }
  }, []);

  const renderBoard = useCallback(
    (animateTransitions: boolean) => {
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container || !board) {
        return;
      }

      cancelAnimation();
      const setup = setupCanvas(canvas, container);
      if (!setup) {
        return;
      }

      const previous = previousRenderRef.current;
      const cellCount = board.cells.length;
      const expectedBirths = latestTick?.iteration === board.iteration
        ? boundedCount(latestTick.births, cellCount)
        : 0;
      const expectedDeaths = latestTick?.iteration === board.iteration
        ? boundedCount(latestTick.deaths, cellCount)
        : 0;
      const canAnimate =
        animateTransitions &&
        previous !== null &&
        previous.width === board.width &&
        previous.height === board.height &&
        board.iteration === previous.iteration + 1 &&
        (expectedBirths > 0 || expectedDeaths > 0) &&
        paletteHasTransitionColors(palette);

      const overlay = drawBoard(
        setup.ctx,
        setup.cssWidth,
        setup.cssHeight,
        board.width,
        board.height,
        board.cells,
        palette,
        canAnimate
          ? {
              previousCells: previous.cells,
              expectedBirths,
              expectedDeaths,
            }
          : undefined,
      );

      previousRenderRef.current = {
        width: board.width,
        height: board.height,
        iteration: board.iteration,
        cells: board.cells,
      };

      if (overlay && (overlay.births.length > 0 || overlay.deaths.length > 0)) {
        startTransitionAnimation(setup.ctx, overlay, animationFrameRef);
      }
    },
    [board, cancelAnimation, latestTick, palette],
  );
  renderBoardRef.current = renderBoard;

  useEffect(() => {
    if (!board) {
      previousRenderRef.current = null;
      cancelAnimation();
      return;
    }
    renderBoard(true);
    return cancelAnimation;
  }, [board, cancelAnimation, renderBoard]);

  // Redraw on container resize so the cells scale to fill the available
  // space without distorting.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }
    let lastWidth = container.clientWidth;
    let lastHeight = container.clientHeight;
    const observer = new ResizeObserver(() => {
      const current = containerRef.current;
      if (!current) {
        return;
      }
      const nextWidth = current.clientWidth;
      const nextHeight = current.clientHeight;
      if (nextWidth === lastWidth && nextHeight === lastHeight) {
        return;
      }
      lastWidth = nextWidth;
      lastHeight = nextHeight;
      renderBoardRef.current(false);
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const editable = sessionMode === "setup" && !readOnly;

  const cellAt = (clientX: number, clientY: number) => {
    const canvas = canvasRef.current;
    if (!canvas || !board) {
      return null;
    }
    const rect = canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    const { cellSize, offsetX, offsetY } = layoutFor(rect.width, rect.height, board.width, board.height);
    const cellX = Math.floor((x - offsetX) / cellSize);
    const cellY = Math.floor((y - offsetY) / cellSize);
    if (cellX < 0 || cellY < 0 || cellX >= board.width || cellY >= board.height) {
      return null;
    }
    return { x: cellX, y: cellY };
  };

  const onPointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!editable || !board) {
      return;
    }
    const cell = cellAt(event.clientX, event.clientY);
    if (!cell) {
      return;
    }
    const current = board.cells[cell.y * board.width + cell.x];
    const paintingAlive = current === 0;
    dragRef.current = { paintingAlive, lastX: cell.x, lastY: cell.y };
    void setCellAction(cell.x, cell.y, paintingAlive);
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!editable || !dragRef.current) {
      return;
    }
    const cell = cellAt(event.clientX, event.clientY);
    if (!cell) {
      return;
    }
    if (cell.x === dragRef.current.lastX && cell.y === dragRef.current.lastY) {
      return;
    }
    dragRef.current.lastX = cell.x;
    dragRef.current.lastY = cell.y;
    void setCellAction(cell.x, cell.y, dragRef.current.paintingAlive);
  };

  const endDrag = (event: React.PointerEvent<HTMLCanvasElement>) => {
    dragRef.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  };

  return (
    <div ref={containerRef} className={styles.root}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        style={{ cursor: editable ? "crosshair" : "not-allowed" }}
        data-readonly={editable ? "false" : "true"}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        role="img"
        aria-label={`Game of Life board ${board?.width ?? 0} by ${board?.height ?? 0}`}
      />
    </div>
  );
};

interface Layout {
  cellSize: number;
  offsetX: number;
  offsetY: number;
}

interface CanvasSetup {
  ctx: CanvasRenderingContext2D;
  cssWidth: number;
  cssHeight: number;
}

interface DrawBoardOptions {
  previousCells: Uint8Array;
  expectedBirths: number;
  expectedDeaths: number;
}

interface TransitionOverlay extends Layout {
  boardWidth: number;
  boardHeight: number;
  births: number[];
  deaths: number[];
  alive: string;
  dead: string;
  birth: string;
  death: string;
  grid: string;
}

const setupCanvas = (
  canvas: HTMLCanvasElement,
  container: HTMLDivElement,
): CanvasSetup | null => {
  const dpr = window.devicePixelRatio || 1;
  const cssWidth = container.clientWidth;
  const cssHeight = container.clientHeight;
  canvas.style.width = `${cssWidth}px`;
  canvas.style.height = `${cssHeight}px`;
  canvas.width = Math.floor(cssWidth * dpr);
  canvas.height = Math.floor(cssHeight * dpr);

  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return null;
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  return { ctx, cssWidth, cssHeight };
};

const boundedCount = (value: number, max: number): number => {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(max, Math.trunc(value)));
};

const paletteHasTransitionColors = (palette: CellPalette): boolean =>
  palette.resurrecting !== palette.alive || palette.dying !== palette.dead;

const layoutFor = (
  cssWidth: number,
  cssHeight: number,
  boardWidth: number,
  boardHeight: number,
): Layout => {
  if (boardWidth === 0 || boardHeight === 0) {
    return { cellSize: 0, offsetX: 0, offsetY: 0 };
  }
  const cellSize = Math.max(1, Math.floor(Math.min(cssWidth / boardWidth, cssHeight / boardHeight)));
  const drawWidth = cellSize * boardWidth;
  const drawHeight = cellSize * boardHeight;
  const offsetX = Math.floor((cssWidth - drawWidth) / 2);
  const offsetY = Math.floor((cssHeight - drawHeight) / 2);
  return { cellSize, offsetX, offsetY };
};

const drawBoard = (
  ctx: CanvasRenderingContext2D,
  cssWidth: number,
  cssHeight: number,
  boardWidth: number,
  boardHeight: number,
  cells: Uint8Array,
  palette: CellPalette,
  options?: DrawBoardOptions,
): TransitionOverlay | null => {
  ctx.fillStyle = palette.background;
  ctx.fillRect(0, 0, cssWidth, cssHeight);

  const { cellSize, offsetX, offsetY } = layoutFor(cssWidth, cssHeight, boardWidth, boardHeight);
  if (cellSize === 0) {
    return null;
  }

  // Whole board dead background.
  ctx.fillStyle = palette.dead;
  ctx.fillRect(offsetX, offsetY, cellSize * boardWidth, cellSize * boardHeight);

  const births: number[] = [];
  const deaths: number[] = [];
  const previousCells = options?.previousCells;
  const expectedBirths = options?.expectedBirths ?? 0;
  const expectedDeaths = options?.expectedDeaths ?? 0;

  // Alive cells. Transition detection is fused into this traversal so we
  // don't add a separate full-board diff pass for large boards.
  for (let y = 0; y < boardHeight; y += 1) {
    for (let x = 0; x < boardWidth; x += 1) {
      const index = y * boardWidth + x;
      const alive = cells[index] !== 0;
      if (alive) {
        ctx.fillStyle = palette.alive;
        ctx.fillRect(offsetX + x * cellSize, offsetY + y * cellSize, cellSize, cellSize);
      }
      if (
        previousCells &&
        (births.length < expectedBirths || deaths.length < expectedDeaths)
      ) {
        const wasAlive = previousCells[index] !== 0;
        if (alive && !wasAlive && births.length < expectedBirths) {
          births.push(index);
        } else if (!alive && wasAlive && deaths.length < expectedDeaths) {
          deaths.push(index);
        }
      }
    }
  }

  drawGrid(ctx, { cellSize, offsetX, offsetY }, boardWidth, boardHeight, palette.grid);

  if (!previousCells || (births.length === 0 && deaths.length === 0)) {
    return null;
  }
  return {
    cellSize,
    offsetX,
    offsetY,
    boardWidth,
    boardHeight,
    births,
    deaths,
    alive: palette.alive,
    dead: palette.dead,
    birth: palette.resurrecting,
    death: palette.dying,
    grid: palette.grid,
  };
};

const drawGrid = (
  ctx: CanvasRenderingContext2D,
  layout: Layout,
  boardWidth: number,
  boardHeight: number,
  gridColor: string,
) => {
  const { cellSize, offsetX, offsetY } = layout;
  if (cellSize >= 6) {
    ctx.strokeStyle = gridColor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = 0; x <= boardWidth; x += 1) {
      const px = Math.floor(offsetX + x * cellSize) + 0.5;
      ctx.moveTo(px, offsetY);
      ctx.lineTo(px, offsetY + cellSize * boardHeight);
    }
    for (let y = 0; y <= boardHeight; y += 1) {
      const py = Math.floor(offsetY + y * cellSize) + 0.5;
      ctx.moveTo(offsetX, py);
      ctx.lineTo(offsetX + cellSize * boardWidth, py);
    }
    ctx.stroke();
  }
};

const startTransitionAnimation = (
  ctx: CanvasRenderingContext2D,
  overlay: TransitionOverlay,
  frameRef: React.MutableRefObject<number | null>,
) => {
  const startedAt = performance.now();
  drawTransitionFrame(ctx, overlay, 0);

  const tick = (now: number) => {
    const progress = Math.min(1, (now - startedAt) / TRANSITION_DURATION_MS);
    drawTransitionFrame(ctx, overlay, easeOutCubic(progress));
    if (progress < 1) {
      frameRef.current = window.requestAnimationFrame(tick);
    } else {
      frameRef.current = null;
    }
  };

  frameRef.current = window.requestAnimationFrame(tick);
};

const drawTransitionFrame = (
  ctx: CanvasRenderingContext2D,
  overlay: TransitionOverlay,
  progress: number,
) => {
  const birthColor = mixColor(overlay.birth, overlay.alive, progress);
  const deathColor = mixColor(overlay.death, overlay.dead, progress);
  for (const index of overlay.births) {
    drawTransitionCell(ctx, overlay, index, birthColor);
  }
  for (const index of overlay.deaths) {
    drawTransitionCell(ctx, overlay, index, deathColor);
  }
  drawGrid(ctx, overlay, overlay.boardWidth, overlay.boardHeight, overlay.grid);
};

const drawTransitionCell = (
  ctx: CanvasRenderingContext2D,
  overlay: TransitionOverlay,
  index: number,
  color: string,
) => {
  const x = index % overlay.boardWidth;
  const y = Math.floor(index / overlay.boardWidth);
  ctx.fillStyle = color;
  ctx.fillRect(
    overlay.offsetX + x * overlay.cellSize,
    overlay.offsetY + y * overlay.cellSize,
    overlay.cellSize,
    overlay.cellSize,
  );
};

const easeOutCubic = (progress: number): number => 1 - (1 - progress) ** 3;

const mixColor = (from: string, to: string, progress: number): string => {
  const start = parseHexColor(from);
  const end = parseHexColor(to);
  if (!start || !end) {
    return progress < 1 ? from : to;
  }
  const channel = (a: number, b: number) =>
    Math.round(a + (b - a) * progress)
      .toString(16)
      .padStart(2, "0");
  return `#${channel(start.r, end.r)}${channel(start.g, end.g)}${channel(start.b, end.b)}`;
};

const parseHexColor = (value: string): { r: number; g: number; b: number } | null => {
  const match = /^#([\da-f]{2})([\da-f]{2})([\da-f]{2})$/i.exec(value);
  if (!match) {
    return null;
  }
  return {
    r: Number.parseInt(match[1], 16),
    g: Number.parseInt(match[2], 16),
    b: Number.parseInt(match[3], 16),
  };
};
