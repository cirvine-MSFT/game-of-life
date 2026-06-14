import { useEffect } from "react";
import { Body1, Caption1, makeStyles, tokens } from "@fluentui/react-components";

import { BoardCanvas } from "./BoardCanvas";
import { PlaybackControls } from "./PlaybackControls";
import { StatsPanel } from "./StatsPanel";
import { useStore, type ThemeChoice } from "../state/store";

const useStyles = makeStyles({
  root: {
    display: "grid",
    gridTemplateRows: "auto 1fr auto",
    height: "100%",
    backgroundColor: tokens.colorNeutralBackground2,
    color: tokens.colorNeutralForeground1,
  },
  body: {
    display: "grid",
    gridTemplateColumns: "1fr auto",
    minHeight: 0,
  },
  canvas: {
    minWidth: 0,
    minHeight: 0,
    display: "flex",
    alignItems: "stretch",
    justifyContent: "stretch",
    padding: tokens.spacingHorizontalM,
  },
  statusBar: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: `${tokens.spacingVerticalXS} ${tokens.spacingHorizontalM}`,
    backgroundColor: tokens.colorNeutralBackground1,
    borderTop: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  initError: {
    color: tokens.colorPaletteRedForeground1,
  },
});

const paletteNameFor = (theme: ThemeChoice): "light" | "dark" | "highContrast" => {
  switch (theme) {
    case "dark":
      return "dark";
    case "highContrast":
      return "highContrast";
    default:
      return "light";
  }
};

/**
 * Root layout. Three rows (toolbar / body / status bar); the body
 * splits into the centered BoardCanvas and a collapsible right-side
 * StatsPanel.
 *
 * `connect()` is called once on mount. The store guards against
 * double-subscription so React 19 strict-mode double-mounting is safe.
 */
export const Shell = () => {
  const styles = useStyles();
  const connect = useStore((s) => s.connect);
  const initError = useStore((s) => s.initError);
  const session = useStore((s) => s.session);
  const theme = useStore((s) => s.theme);

  useEffect(() => {
    void connect();
  }, [connect]);

  if (initError) {
    return (
      <div className={styles.root}>
        <Body1 className={styles.initError}>
          Failed to connect to the simulation backend: {initError}
        </Body1>
      </div>
    );
  }

  return (
    <div className={styles.root}>
      <PlaybackControls />
      <div className={styles.body}>
        <div className={styles.canvas}>
          <BoardCanvas paletteName={paletteNameFor(theme)} />
        </div>
        <StatsPanel />
      </div>
      <div className={styles.statusBar}>
        <Caption1>
          {session
            ? `Mode: ${session.mode} \u00b7 ${session.width}\u00d7${session.height} \u00b7 Max iterations: ${session.maxIterations}`
            : "Connecting\u2026"}
        </Caption1>
        <Caption1>
          {session?.savePath ? `Save path: ${session.savePath}` : "Unsaved"}
        </Caption1>
      </div>
    </div>
  );
};
