import { useEffect } from "react";
import {
  Body1,
  Button,
  Caption1,
  makeStyles,
  tokens,
} from "@fluentui/react-components";

import { NavRail } from "./NavRail";
import {
  AggregatePanePlaceholder,
  EditPanePlaceholder,
  RunPanePlaceholder,
  SettingsPanePlaceholder,
} from "../panes/PanePlaceholders";
import { useStore, type ActiveView } from "../state/store";

const useStyles = makeStyles({
  root: {
    display: "grid",
    gridTemplateColumns: "auto 1fr",
    height: "100%",
    backgroundColor: tokens.colorNeutralBackground2,
    color: tokens.colorNeutralForeground1,
  },
  body: {
    display: "grid",
    gridTemplateRows: "1fr auto",
    minHeight: 0,
  },
  content: {
    minHeight: 0,
    overflow: "auto",
    padding: tokens.spacingHorizontalM,
  },
  statusBar: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: `${tokens.spacingVerticalXS} ${tokens.spacingHorizontalM}`,
    backgroundColor: tokens.colorNeutralBackground1,
    borderTop: `1px solid ${tokens.colorNeutralStroke2}`,
    gap: tokens.spacingHorizontalM,
  },
  initError: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    height: "100%",
    gap: tokens.spacingVerticalM,
    padding: tokens.spacingHorizontalXL,
    textAlign: "center",
    color: tokens.colorPaletteRedForeground1,
  },
});

const paneFor = (view: ActiveView) => {
  switch (view) {
    case "edit":
      return <EditPanePlaceholder />;
    case "run":
      return <RunPanePlaceholder />;
    case "aggregate":
      return <AggregatePanePlaceholder />;
    case "settings":
      return <SettingsPanePlaceholder />;
    // Telemetry is reachable only via a disabled rail tab; the store's
    // persistence load also coerces "telemetry" to "edit" so this branch is
    // effectively unreachable at runtime. Render the Edit placeholder
    // defensively rather than throwing.
    case "telemetry":
    default:
      return <EditPanePlaceholder />;
  }
};

const statusBarLeft = (
  view: ActiveView,
  session: ReturnType<typeof useStore.getState>["session"],
): string => {
  if (!session) return "Connecting\u2026";
  switch (view) {
    case "aggregate":
      return "Aggregate view";
    case "settings":
      return "Settings";
    case "edit":
    case "run":
    default:
      return `Mode: ${session.mode} \u00b7 ${session.width}\u00d7${session.height} \u00b7 Iter ${session.iteration}`;
  }
};

const statusBarRight = (
  view: ActiveView,
  session: ReturnType<typeof useStore.getState>["session"],
  theme: string,
): string => {
  switch (view) {
    case "settings":
      return `Theme: ${theme}`;
    case "aggregate":
      return "";
    case "edit":
    case "run":
    default:
      return session?.savePath ? `Save path: ${session.savePath}` : "Unsaved";
  }
};

/**
 * Root layout for the revamped UI. A persistent left-side `NavRail`
 * switches between four active destinations (Edit / Run / Aggregate /
 * Settings) and a fifth disabled "Telemetry" slot reserved for a future
 * cross-instance OTel view. Each destination renders into the same
 * content area; a bottom status bar carries pane-specific context.
 *
 * `connect()` is called once on mount; the store guards against
 * double-subscription so React strict-mode double-mounting is safe.
 */
export const AppShell = () => {
  const styles = useStyles();
  const connect = useStore((s) => s.connect);
  const initError = useStore((s) => s.initError);
  const session = useStore((s) => s.session);
  const activeView = useStore((s) => s.activeView);
  const theme = useStore((s) => s.theme);

  useEffect(() => {
    void connect();
  }, [connect]);

  if (initError) {
    return (
      <div className={styles.root}>
        <NavRail />
        <div className={styles.body}>
          <div className={styles.initError}>
            <Body1>Failed to connect to the simulation backend.</Body1>
            <Body1>{initError}</Body1>
            <Button appearance="primary" onClick={() => void connect()}>
              Retry
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.root}>
      <NavRail />
      <div className={styles.body}>
        <div className={styles.content}>{paneFor(activeView)}</div>
        <div className={styles.statusBar} aria-label="Status bar">
          <Caption1>{statusBarLeft(activeView, session)}</Caption1>
          <Caption1>{statusBarRight(activeView, session, theme)}</Caption1>
        </div>
      </div>
    </div>
  );
};
