import { useState } from "react";
import {
  Body1,
  Button,
  Caption1,
  Subtitle2,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { useStore } from "../state/store";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    backgroundColor: tokens.colorNeutralBackground1,
    borderLeft: `1px solid ${tokens.colorNeutralStroke2}`,
    width: "260px",
    minWidth: "260px",
    overflowY: "auto",
  },
  collapsed: {
    width: "40px",
    minWidth: "40px",
    alignItems: "center",
    padding: tokens.spacingVerticalM,
  },
  expanded: {
    padding: tokens.spacingVerticalM,
    gap: tokens.spacingVerticalM,
  },
  metricsGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: tokens.spacingHorizontalS,
  },
  metric: {
    display: "flex",
    flexDirection: "column",
  },
  chart: {
    height: "180px",
    width: "100%",
  },
  toggleButton: {
    alignSelf: "flex-end",
  },
});

const HISTORY_DECIMATION_TARGET = 200;

/**
 * Decimates the alive-count history down to roughly
 * `HISTORY_DECIMATION_TARGET` points so Recharts stays responsive even
 * after thousands of generations. We tag each point with its absolute
 * generation index so the chart's x-axis reflects real time, not the
 * decimated index.
 */
const prepareSeries = (history: number[]): { generation: number; alive: number }[] => {
  if (history.length <= HISTORY_DECIMATION_TARGET) {
    return history.map((alive, generation) => ({ generation, alive }));
  }
  const stride = Math.ceil(history.length / HISTORY_DECIMATION_TARGET);
  const out: { generation: number; alive: number }[] = [];
  for (let i = 0; i < history.length; i += stride) {
    out.push({ generation: i, alive: history[i] });
  }
  // Always include the most recent point so the chart's right edge
  // matches the current iteration.
  if (out[out.length - 1]?.generation !== history.length - 1) {
    out.push({ generation: history.length - 1, alive: history[history.length - 1] });
  }
  return out;
};

export const StatsPanel = () => {
  const styles = useStyles();
  const [collapsed, setCollapsed] = useState(false);
  const history = useStore((s) => s.history);
  const latestTick = useStore((s) => s.latestTick);
  const session = useStore((s) => s.session);
  const finalStats = useStore((s) => s.finalStats);

  if (collapsed) {
    return (
      <aside className={`${styles.root} ${styles.collapsed}`}>
        <Button appearance="subtle" onClick={() => setCollapsed(false)} aria-label="Expand stats panel">
          ◀
        </Button>
      </aside>
    );
  }

  const series = prepareSeries(history);
  const alive = latestTick?.alive ?? history[history.length - 1] ?? 0;
  const dead = latestTick?.dead;
  const births = latestTick?.births ?? 0;
  const deaths = latestTick?.deaths ?? 0;

  return (
    <aside className={`${styles.root} ${styles.expanded}`}>
      <div className={styles.toggleButton}>
        <Button
          appearance="subtle"
          onClick={() => setCollapsed(true)}
          aria-label="Collapse stats panel"
        >
          ▶
        </Button>
      </div>

      <Subtitle2>Generation {session?.iteration ?? 0}</Subtitle2>

      <div className={styles.metricsGrid}>
        <div className={styles.metric}>
          <Caption1>Alive</Caption1>
          <Body1>{alive}</Body1>
        </div>
        <div className={styles.metric}>
          <Caption1>Dead</Caption1>
          <Body1>{dead ?? "—"}</Body1>
        </div>
        <div className={styles.metric}>
          <Caption1>Births</Caption1>
          <Body1>{births}</Body1>
        </div>
        <div className={styles.metric}>
          <Caption1>Deaths</Caption1>
          <Body1>{deaths}</Body1>
        </div>
      </div>

      <Subtitle2>Alive over time</Subtitle2>
      <div className={styles.chart}>
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={series}>
            <CartesianGrid strokeDasharray="3 3" stroke={tokens.colorNeutralStroke2} />
            <XAxis dataKey="generation" stroke={tokens.colorNeutralForeground3} />
            <YAxis allowDecimals={false} stroke={tokens.colorNeutralForeground3} />
            <Tooltip />
            <Line
              type="monotone"
              dataKey="alive"
              stroke={tokens.colorBrandForeground1}
              strokeWidth={2}
              dot={false}
              isAnimationActive={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {finalStats && (
        <>
          <Subtitle2>Run summary</Subtitle2>
          <div className={styles.metricsGrid}>
            <div className={styles.metric}>
              <Caption1>Status</Caption1>
              <Body1>{finalStats.status}</Body1>
            </div>
            <div className={styles.metric}>
              <Caption1>Iterations</Caption1>
              <Body1>{finalStats.iterationsRun}</Body1>
            </div>
            <div className={styles.metric}>
              <Caption1>Peak alive</Caption1>
              <Body1>{finalStats.peakAliveCount}</Body1>
            </div>
            <div className={styles.metric}>
              <Caption1>Total births</Caption1>
              <Body1>{finalStats.totalBirths}</Body1>
            </div>
          </div>
        </>
      )}
    </aside>
  );
};
