import { useEffect, useRef } from "react";
import { makeStyles, tokens } from "@fluentui/react-components";

import { useStore } from "../state/store";
import { paletteFor, type PaletteName } from "../theme";

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
}

interface DragState {
  /** The alive-value the drag started with; we paint a continuous stroke
   *  of the opposite of the cell that was clicked, so a single drag
   *  feels like a toggle-and-extend rather than an unpredictable mix. */
  paintingAlive: boolean;
  lastX: number;
  lastY: number;
}

/**
 * Renders the current board onto a Canvas 2D surface and wires
 * pointer events for click + drag-paint when the session is in Setup
 * mode. Animation of transitional states lands later (`animation`
 * todo); for now cells flip hard between dead and alive on each
 * board-tick.
 */
export const BoardCanvas = ({ paletteName }: BoardCanvasProps) => {
  const styles = useStyles();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const dragRef = useRef<DragState | null>(null);
  const board = useStore((s) => s.board);
  const sessionMode = useStore((s) => s.session?.mode ?? "setup");
  const setCellAction = useStore((s) => s.setCell);

  const palette = paletteFor(paletteName);

  // Redraw whenever the board snapshot or palette changes. Cells are
  // packed Dead=0, Alive=1 in row-major order matching the Rust shadow
  // buffer.
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || !board) {
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    const cssWidth = container.clientWidth;
    const cssHeight = container.clientHeight;
    canvas.style.width = `${cssWidth}px`;
    canvas.style.height = `${cssHeight}px`;
    canvas.width = Math.floor(cssWidth * dpr);
    canvas.height = Math.floor(cssHeight * dpr);

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    drawBoard(ctx, cssWidth, cssHeight, board.width, board.height, board.cells, palette);
  }, [board, palette]);

  // Redraw on container resize so the cells scale to fill the available
  // space without distorting.
  useEffect(() => {
    if (!containerRef.current) {
      return;
    }
    const observer = new ResizeObserver(() => {
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container || !board) {
        return;
      }
      const dpr = window.devicePixelRatio || 1;
      canvas.style.width = `${container.clientWidth}px`;
      canvas.style.height = `${container.clientHeight}px`;
      canvas.width = Math.floor(container.clientWidth * dpr);
      canvas.height = Math.floor(container.clientHeight * dpr);
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        return;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      drawBoard(
        ctx,
        container.clientWidth,
        container.clientHeight,
        board.width,
        board.height,
        board.cells,
        palette,
      );
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [board, palette]);

  const editable = sessionMode === "setup";

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
  palette: { dead: string; alive: string; grid: string; background: string },
) => {
  ctx.fillStyle = palette.background;
  ctx.fillRect(0, 0, cssWidth, cssHeight);

  const { cellSize, offsetX, offsetY } = layoutFor(cssWidth, cssHeight, boardWidth, boardHeight);
  if (cellSize === 0) {
    return;
  }

  // Whole board dead background.
  ctx.fillStyle = palette.dead;
  ctx.fillRect(offsetX, offsetY, cellSize * boardWidth, cellSize * boardHeight);

  // Alive cells.
  ctx.fillStyle = palette.alive;
  for (let y = 0; y < boardHeight; y += 1) {
    for (let x = 0; x < boardWidth; x += 1) {
      if (cells[y * boardWidth + x] !== 0) {
        ctx.fillRect(offsetX + x * cellSize, offsetY + y * cellSize, cellSize, cellSize);
      }
    }
  }

  // Grid lines, only when cells are large enough to be readable.
  if (cellSize >= 6) {
    ctx.strokeStyle = palette.grid;
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
