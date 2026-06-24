// Shared formatter for terminal-state UX across PlaybackControls,
// StatsPanel, and the status bar. Centralising the copy + Fluent badge
// colour keeps all three surfaces from drifting apart visually.

import type { IpcRunStatus } from "../ipc";

// Fluent <Badge> accepts a fixed set of semantic colours. We only use the
// four that make sense for these outcomes.
export type TerminalBadgeColor = "success" | "brand" | "severe" | "warning";

export interface TerminalStatusDescriptor {
  // Short text suitable for a Badge ("Stable", "Cyclic", ...).
  shortLabel: string;
  // Full text including the generation it stopped at, suitable for the
  // status bar / panel labels ("Stable at gen 12").
  label: string;
  // One-sentence description for tooltips and aria-label.
  description: string;
  // Fluent <Badge color={...}> value.
  color: TerminalBadgeColor;
}

export interface TerminalCycleInfo {
  // Period from `IpcRunStatistics.cyclePeriod`; required to enrich the
  // "Cyclic" label/description.
  period?: number | null;
  startGeneration?: number | null;
}

export const formatTerminalStatus = (
  status: IpcRunStatus,
  iteration: number,
  cycle?: TerminalCycleInfo | null,
): TerminalStatusDescriptor => {
  switch (status) {
    case "stable":
      return {
        shortLabel: "Stable",
        label: `Stable at gen ${iteration}`,
        description: `Run stopped because the board reached a fixed-point stable state at generation ${iteration}.`,
        color: "success",
      };
    case "cyclic": {
      const period = cycle?.period ?? null;
      const periodSuffix = period != null ? ` (period ${period})` : "";
      return {
        shortLabel: "Cyclic",
        label: `Cyclic at gen ${iteration}${periodSuffix}`,
        description:
          period != null
            ? `Run stopped because the board entered a cycle of period ${period} (detected at generation ${iteration}).`
            : `Run stopped because the board entered a cycle (detected at generation ${iteration}).`,
        color: "brand",
      };
    }
    case "extinct":
      return {
        shortLabel: "Extinct",
        label: `Extinct at gen ${iteration}`,
        description: `Run stopped because every cell died at generation ${iteration}.`,
        color: "severe",
      };
    case "maxIterations":
      return {
        shortLabel: "Reached max",
        label: `Reached max (${iteration})`,
        description: `Run stopped after reaching the configured maximum of ${iteration} generations.`,
        color: "warning",
      };
  }
};
