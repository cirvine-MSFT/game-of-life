import type { IpcRunStatistics, IpcRunStatus, SessionInfo } from "../ipc";

export type TerminalBadgeColor = "success" | "brand" | "severe" | "warning";

export interface TerminalStatusDescriptor {
  shortLabel: string;
  label: string;
  description: string;
  color: TerminalBadgeColor;
}

export interface TerminalCycleInfo {
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
      const startGeneration = cycle?.startGeneration ?? null;
      const periodSuffix = period != null ? ` (period ${period})` : "";
      const startSuffix = startGeneration != null ? `, first seen at generation ${startGeneration}` : "";
      return {
        shortLabel: "Cyclic",
        label: `Cyclic at gen ${iteration}${periodSuffix}`,
        description:
          period != null
            ? `Run stopped because the board entered a cycle of period ${period} (detected at generation ${iteration}${startSuffix}).`
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

export const cycleInfoFromStats = (
  stats: IpcRunStatistics | null | undefined,
): TerminalCycleInfo | null =>
  stats
    ? {
        period: stats.cyclePeriod ?? null,
        startGeneration: stats.cycleStartGeneration ?? null,
      }
    : null;

export const formatTerminalStatusFromStats = (
  stats: IpcRunStatistics,
): TerminalStatusDescriptor =>
  formatTerminalStatus(stats.status, stats.iterationsRun, cycleInfoFromStats(stats));

export const formatTerminalStatusFromSession = (
  session: SessionInfo,
  finalStats: IpcRunStatistics | null | undefined,
): TerminalStatusDescriptor | null =>
  session.completed && session.status
    ? formatTerminalStatus(
        session.status,
        finalStats?.iterationsRun ?? session.iteration,
        cycleInfoFromStats(finalStats),
      )
    : null;
