import { useMemo, useState } from "react";
import {
  Badge,
  Body1,
  Button,
  Caption1,
  Checkbox,
  Dropdown,
  Option,
  Spinner,
  Subtitle2,
  Table,
  TableBody,
  TableCell,
  TableHeader,
  TableHeaderCell,
  TableRow,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { useStore, type AggregateRow } from "../state/store";
import { decimatedIndices } from "../state/seriesDecimation";
import { formatTerminalStatusFromStats } from "../state/terminalStatus";

export const AGGREGATE_COLORS = [
  "#2563eb",
  "#16a34a",
  "#dc2626",
  "#9333ea",
  "#ea580c",
  "#0891b2",
  "#be123c",
  "#4f46e5",
  "#65a30d",
  "#c026d3",
] as const;

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalL,
    minHeight: 0,
  },
  card: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
    padding: tokens.spacingHorizontalL,
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: tokens.borderRadiusLarge,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  headerRow: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: tokens.spacingHorizontalM,
    flexWrap: "wrap",
  },
  actions: {
    display: "flex",
    alignItems: "center",
    gap: tokens.spacingHorizontalS,
  },
  fileList: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
  },
  fileRow: {
    display: "grid",
    gridTemplateColumns: "auto minmax(180px, 1fr) auto auto auto",
    alignItems: "center",
    gap: tokens.spacingHorizontalS,
    padding: tokens.spacingHorizontalS,
    borderRadius: tokens.borderRadiusMedium,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
    "@media (max-width: 760px)": {
      gridTemplateColumns: "auto minmax(0, 1fr) auto",
    },
  },
  swatch: {
    width: "12px",
    height: "12px",
    borderRadius: tokens.borderRadiusCircular,
    border: `1px solid ${tokens.colorNeutralStroke1}`,
  },
  filename: {
    display: "flex",
    flexDirection: "column",
    minWidth: 0,
  },
  muted: {
    color: tokens.colorNeutralForeground3,
  },
  errorText: {
    color: tokens.colorPaletteRedForeground1,
  },
  charts: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalL,
  },
  chart: {
    height: "260px",
    width: "100%",
  },
  chartHeader: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: tokens.spacingHorizontalM,
    flexWrap: "wrap",
  },
  dropdown: {
    minWidth: "220px",
  },
  emptyState: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalXS,
    padding: `${tokens.spacingVerticalM} 0`,
  },
});

interface ChartDatum {
  generation: number;
  [key: string]: number;
}

const colorFor = (colorIndex: number): string =>
  AGGREGATE_COLORS[colorIndex % AGGREGATE_COLORS.length];

const statusLabel = (row: AggregateRow): string => {
  switch (row.status) {
    case "loading":
      return "Loading";
    case "ready":
      return "Ready";
    case "summaryOnly":
      return "Summary only";
    case "error":
      return "Error";
  }
};

const statusColor = (
  row: AggregateRow,
): "brand" | "danger" | "informative" | "success" | "warning" => {
  switch (row.status) {
    case "loading":
      return "informative";
    case "ready":
      return "success";
    case "summaryOnly":
      return "warning";
    case "error":
      return "danger";
  }
};

const buildAliveOverlayData = (rows: AggregateRow[]): ChartDatum[] => {
  const points = new Map<number, ChartDatum>();
  rows.forEach((row, rowIndex) => {
    if (!row.series) {
      return;
    }
    const dataKey = `run_${rowIndex}`;
    for (const generation of decimatedIndices(row.series.alive.length)) {
      const point = points.get(generation) ?? { generation };
      point[dataKey] = row.series.alive[generation];
      points.set(generation, point);
    }
  });
  return [...points.values()].sort((a, b) => a.generation - b.generation);
};

const buildBirthDeathData = (row: AggregateRow | undefined): ChartDatum[] => {
  if (!row?.series) {
    return [];
  }
  const length = Math.max(row.series.births.length, row.series.deaths.length);
  return decimatedIndices(length).map((generation) => ({
    generation,
    births: row.series?.births[generation] ?? 0,
    deaths: row.series?.deaths[generation] ?? 0,
  }));
};

const cyclePeriodLabel = (row: AggregateRow): string => {
  const period = row.summary?.cyclePeriod;
  return period == null ? "—" : String(period);
};

export const AggregatePane = () => {
  const styles = useStyles();
  const rows = useStore((s) => s.aggregateRows);
  const addAggregateFiles = useStore((s) => s.addAggregateFiles);
  const clearAggregate = useStore((s) => s.clearAggregate);
  const removeAggregateRow = useStore((s) => s.removeAggregateRow);
  const setAggregateRowVisible = useStore((s) => s.setAggregateRowVisible);
  const [filePickerError, setFilePickerError] = useState<string | null>(null);
  const [selectedRowPath, setSelectedRowPath] = useState<string | null>(null);

  const readyRows = useMemo(
    () => rows.filter((row) => row.status === "ready" && row.series),
    [rows],
  );
  const visibleReadyRows = useMemo(
    () => readyRows.filter((row) => row.visible),
    [readyRows],
  );
  const summaryRows = useMemo(
    () =>
      rows.filter(
        (row) =>
          (row.status === "ready" || row.status === "summaryOnly") && row.summary,
      ),
    [rows],
  );

  const aliveOverlayData = useMemo(
    () => buildAliveOverlayData(visibleReadyRows),
    [visibleReadyRows],
  );
  const effectiveSelectedPath =
    readyRows.find((row) => row.path === selectedRowPath)?.path ??
    readyRows[0]?.path ??
    null;
  const selectedRow = readyRows.find((row) => row.path === effectiveSelectedPath);
  const birthDeathData = useMemo(
    () => buildBirthDeathData(selectedRow),
    [selectedRow],
  );

  const handleAddFiles = async () => {
    setFilePickerError(null);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const chosen = await open({
        title: "Add run files",
        multiple: true,
        filters: [{ name: "Game of Life run", extensions: ["gol"] }],
      });
      if (!chosen) {
        return;
      }
      const paths = (Array.isArray(chosen) ? chosen : [chosen]).filter(
        (path): path is string => typeof path === "string",
      );
      await addAggregateFiles(paths);
    } catch (error) {
      setFilePickerError(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <section className={styles.root} aria-label="Aggregate statistics">
      <div className={styles.card} aria-label="Aggregate file list">
        <div className={styles.headerRow}>
          <div>
            <Subtitle2>Run files</Subtitle2>
            <Caption1 className={styles.muted}>
              Compare saved .gol run records from this session.
            </Caption1>
          </div>
          <div className={styles.actions}>
            <Button appearance="primary" onClick={() => void handleAddFiles()}>
              Add files
            </Button>
            <Button disabled={rows.length === 0} onClick={clearAggregate}>
              Clear
            </Button>
          </div>
        </div>

        {filePickerError && (
          <Caption1 role="alert" className={styles.errorText}>
            {filePickerError}
          </Caption1>
        )}

        {rows.length === 0 ? (
          <div className={styles.emptyState}>
            <Body1>No files selected</Body1>
            <Caption1 className={styles.muted}>
              Add one or more run records to compare alive counts, births,
              deaths, and terminal summaries.
            </Caption1>
          </div>
        ) : (
          <div className={styles.fileList}>
            {rows.map((row) => (
              <div className={styles.fileRow} key={row.path}>
                <span
                  className={styles.swatch}
                  style={{ backgroundColor: colorFor(row.colorIndex) }}
                  aria-hidden="true"
                />
                <div className={styles.filename}>
                  <Body1>{row.filename}</Body1>
                  <Caption1 className={styles.muted}>{row.path}</Caption1>
                  {row.status === "summaryOnly" && (
                    <Caption1 className={styles.muted}>
                      summary-only — re-run to capture per-generation data
                    </Caption1>
                  )}
                  {row.status === "error" && row.error && (
                    <Caption1 className={styles.errorText}>{row.error}</Caption1>
                  )}
                </div>
                {row.status === "loading" ? (
                  <Spinner size="tiny" label="Loading" />
                ) : (
                  <Badge color={statusColor(row)}>{statusLabel(row)}</Badge>
                )}
                {row.status === "ready" ? (
                  <Checkbox
                    aria-label={`Show ${row.filename} in overlay chart`}
                    checked={row.visible}
                    onChange={(_, data) =>
                      setAggregateRowVisible(row.path, Boolean(data.checked))
                    }
                  />
                ) : (
                  <span />
                )}
                <Button
                  appearance="subtle"
                  aria-label={`Remove ${row.filename}`}
                  onClick={() => removeAggregateRow(row.path)}
                >
                  ×
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className={styles.charts} aria-label="Aggregate charts">
        <div className={styles.card} aria-label="Alive over time chart">
          <div>
            <Subtitle2>Alive over time</Subtitle2>
            <Caption1 className={styles.muted}>
              Overlay of visible ready runs; summary-only and errored files are
              excluded.
            </Caption1>
          </div>
          {visibleReadyRows.length === 0 ? (
            <Caption1 className={styles.muted}>
              No visible per-generation series yet.
            </Caption1>
          ) : (
            <div className={styles.chart}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={aliveOverlayData}>
                  <CartesianGrid strokeDasharray="3 3" stroke={tokens.colorNeutralStroke2} />
                  <XAxis dataKey="generation" stroke={tokens.colorNeutralForeground3} />
                  <YAxis allowDecimals={false} stroke={tokens.colorNeutralForeground3} />
                  <Tooltip />
                  <Legend />
                  {visibleReadyRows.map((row, index) => (
                    <Line
                      key={row.path}
                      type="monotone"
                      dataKey={`run_${index}`}
                      name={row.filename}
                      stroke={colorFor(row.colorIndex)}
                      strokeWidth={2}
                      dot={false}
                      connectNulls
                      isAnimationActive={false}
                    />
                  ))}
                </LineChart>
              </ResponsiveContainer>
            </div>
          )}
        </div>

        <div className={styles.card} aria-label="Births and deaths chart">
          <div className={styles.chartHeader}>
            <div>
              <Subtitle2>Births / deaths per generation</Subtitle2>
              <Caption1 className={styles.muted}>
                Inspect one ready run at a time to keep the series legible.
              </Caption1>
            </div>
            <Dropdown
              className={styles.dropdown}
              aria-label="Select run for births and deaths"
              disabled={readyRows.length === 0}
              selectedOptions={effectiveSelectedPath ? [effectiveSelectedPath] : []}
              value={selectedRow?.filename ?? ""}
              onOptionSelect={(_, data) => {
                if (data.optionValue) {
                  setSelectedRowPath(data.optionValue);
                }
              }}
            >
              {readyRows.map((row) => (
                <Option key={row.path} value={row.path} text={row.filename}>
                  {row.filename}
                </Option>
              ))}
            </Dropdown>
          </div>
          {selectedRow ? (
            <div className={styles.chart}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={birthDeathData}>
                  <CartesianGrid strokeDasharray="3 3" stroke={tokens.colorNeutralStroke2} />
                  <XAxis dataKey="generation" stroke={tokens.colorNeutralForeground3} />
                  <YAxis allowDecimals={false} stroke={tokens.colorNeutralForeground3} />
                  <Tooltip />
                  <Legend />
                  <Line
                    type="monotone"
                    dataKey="births"
                    name="Births"
                    stroke={tokens.colorPaletteGreenForeground1}
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                  <Line
                    type="monotone"
                    dataKey="deaths"
                    name="Deaths"
                    stroke={tokens.colorPaletteRedForeground1}
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <Caption1 className={styles.muted}>No ready run selected.</Caption1>
          )}
        </div>
      </div>

      <div className={styles.card} aria-label="Aggregate summary table">
        <Subtitle2>Summary</Subtitle2>
        {summaryRows.length === 0 ? (
          <Caption1 className={styles.muted}>
            Loaded summaries will appear here.
          </Caption1>
        ) : (
          <Table aria-label="Aggregate run summary table">
            <TableHeader>
              <TableRow>
                <TableHeaderCell>Filename</TableHeaderCell>
                <TableHeaderCell>Iterations</TableHeaderCell>
                <TableHeaderCell>Status</TableHeaderCell>
                <TableHeaderCell>Peak alive (gen)</TableHeaderCell>
                <TableHeaderCell>Total births</TableHeaderCell>
                <TableHeaderCell>Total deaths</TableHeaderCell>
                <TableHeaderCell>Cycle period</TableHeaderCell>
              </TableRow>
            </TableHeader>
            <TableBody>
              {summaryRows.map((row) => {
                const summary = row.summary;
                if (!summary) {
                  return null;
                }
                const terminal = formatTerminalStatusFromStats(summary);
                return (
                  <TableRow key={row.path}>
                    <TableCell>{row.filename}</TableCell>
                    <TableCell>{summary.iterationsRun}</TableCell>
                    <TableCell>{terminal.shortLabel}</TableCell>
                    <TableCell>
                      {summary.peakAliveCount} (gen {summary.peakAliveGeneration})
                    </TableCell>
                    <TableCell>{summary.totalBirths}</TableCell>
                    <TableCell>{summary.totalDeaths}</TableCell>
                    <TableCell>{cyclePeriodLabel(row)}</TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        )}
      </div>
    </section>
  );
};
