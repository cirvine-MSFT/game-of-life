// Cell-state colour palette plus helpers for the BoardCanvas.
//
// Colours are pulled from Fluent UI's neutral / brand tokens so they
// follow the active theme automatically. The "live" tone needs more
// saturation than the Fluent brand tokens at the default scale, so we
// fall back to fixed hex values when the canvas is rendered outside a
// FluentProvider context (tests, snapshot tooling).

export interface CellPalette {
  dead: string;
  alive: string;
  /** Cell fading from alive -> dead between generations. */
  dying: string;
  /** Cell fading from dead -> alive between generations. */
  resurrecting: string;
  grid: string;
  background: string;
}

export const lightPalette: CellPalette = {
  dead: "#f5f5f5",
  alive: "#0f6cbd",
  dying: "#9d2828",
  resurrecting: "#137a4d",
  grid: "#e1e1e1",
  background: "#fafafa",
};

export const darkPalette: CellPalette = {
  dead: "#1a1a1a",
  alive: "#62a3ff",
  dying: "#d4756b",
  resurrecting: "#5fd49c",
  grid: "#2d2d2d",
  background: "#101010",
};

export const highContrastPalette: CellPalette = {
  dead: "#000000",
  alive: "#ffffff",
  // In high contrast we hide the transitional tints (we also disable
  // fade animations elsewhere) so they collapse to the terminal colour.
  dying: "#000000",
  resurrecting: "#ffffff",
  grid: "#555555",
  background: "#000000",
};

export type PaletteName = "light" | "dark" | "highContrast";

export const paletteFor = (name: PaletteName): CellPalette => {
  switch (name) {
    case "dark":
      return darkPalette;
    case "highContrast":
      return highContrastPalette;
    case "light":
    default:
      return lightPalette;
  }
};
