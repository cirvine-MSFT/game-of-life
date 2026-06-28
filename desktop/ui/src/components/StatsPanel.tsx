import {
  Body1,
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
import { prepareSeries } from "../state/seriesDecimation";
import { formatTerminalStatusFromStats } from "../state/terminalStatus";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
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
});

export const StatsPanel = () => {
  const styles = useStyles();
  const history = useStore((s) => s.history);
  const latestTick = useStore((s) => s.latestTick);
  const session = useStore((s) => s.session);
  const finalStats = useStore((s) => s.finalStats);

  const generation = session?.iteration ?? 0;
  const series = prepareSeries(history);
  const alive = latestTick?.alive ?? history[generation] ?? history[history.length - 1] ?? 0;
  const dead = latestTick?.dead;
  const births = latestTick?.births ?? 0;
  const deaths = latestTick?.deaths ?? 0;
  const terminal = finalStats ? formatTerminalStatusFromStats(finalStats) : null;

  return (
    <section className={styles.root} aria-label="Statistics panel">
      <Subtitle2>Generation {generation}</Subtitle2>

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
              <Body1>{terminal?.shortLabel}</Body1>
            </div>
            <div className={styles.metric}>
              <Caption1>Stopped at generation</Caption1>
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
            {finalStats.cyclePeriod != null && (
              <div className={styles.metric}>
                <Caption1>Cycle period</Caption1>
                <Body1>{finalStats.cyclePeriod}</Body1>
              </div>
            )}
          </div>
        </>
      )}
    </section>
  );
};
