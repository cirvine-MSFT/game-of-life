import {
  Badge,
  Caption1,
  MessageBar,
  MessageBarBody,
  Subtitle2,
  Toolbar,
  ToolbarButton,
  ToolbarDivider,
  makeStyles,
  tokens,
} from "@fluentui/react-components";

import { BoardCanvas } from "../components/BoardCanvas";
import { PlaybackControls } from "../components/PlaybackControls";
import { StatsPanel } from "../components/StatsPanel";
import { useStore, type ThemeChoice } from "../state/store";

const useStyles = makeStyles({
  root: {
    display: "grid",
    gridTemplateColumns: "minmax(420px, 7fr) minmax(300px, 3fr)",
    gap: tokens.spacingHorizontalL,
    height: "100%",
    minHeight: 0,
    "@media (max-width: 960px)": {
      gridTemplateColumns: "1fr",
      height: "auto",
    },
  },
  leftColumn: {
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    gap: tokens.spacingVerticalM,
  },
  toolbar: {
    display: "flex",
    alignItems: "center",
    flexWrap: "wrap",
    gap: tokens.spacingHorizontalS,
    padding: tokens.spacingHorizontalS,
    backgroundColor: tokens.colorNeutralBackground1,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
    borderRadius: tokens.borderRadiusLarge,
  },
  canvasCard: {
    flex: 1,
    minHeight: "420px",
    overflow: "hidden",
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: tokens.borderRadiusLarge,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  statsCard: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
    minHeight: "420px",
    padding: tokens.spacingHorizontalL,
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: tokens.borderRadiusLarge,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  referenceHeader: {
    display: "flex",
    flexDirection: "column",
    alignItems: "flex-start",
    gap: tokens.spacingVerticalS,
  },
  note: {
    color: tokens.colorNeutralForeground3,
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

export const RunPane = () => {
  const styles = useStyles();
  const session = useStore((s) => s.session);
  const theme = useStore((s) => s.theme);
  const loadBoardSnapshot = useStore((s) => s.loadBoardSnapshot);
  const loadSavedRun = useStore((s) => s.loadSavedRun);
  const loadRunBoard = useStore((s) => s.loadRunBoard);
  const loadedReference = useStore((s) => s.loadedReference);

  return (
    <section className={styles.root} aria-label="Run">
      <div className={styles.leftColumn}>
        <Toolbar className={styles.toolbar} aria-label="Run file toolbar">
          <ToolbarButton onClick={() => void loadBoardSnapshot()}>
            Load board
          </ToolbarButton>
          <ToolbarButton onClick={() => void loadSavedRun()}>
            Load saved run
          </ToolbarButton>
          <ToolbarDivider />
          <ToolbarButton onClick={() => void loadRunBoard("final")}>
            Import final board…
          </ToolbarButton>
        </Toolbar>

        <PlaybackControls />

        <div className={styles.canvasCard}>
          <BoardCanvas
            paletteName={paletteNameFor(theme)}
            readOnly={session?.mode !== "setup"}
          />
        </div>
      </div>

      <aside className={styles.statsCard} aria-label="Run statistics">
        <div className={styles.referenceHeader}>
          <Subtitle2>Run statistics</Subtitle2>
          {loadedReference && (
            <Badge appearance="tint" color="informative">
              Loaded reference: {loadedReference.filename}
            </Badge>
          )}
          {loadedReference?.summaryOnly && (
            <MessageBar intent="warning" role="status">
              <MessageBarBody>
                Summary-only run — re-run to capture per-generation data.
              </MessageBarBody>
            </MessageBar>
          )}
          {!loadedReference && (
            <Caption1 className={styles.note}>
              Live statistics update as the simulation runs.
            </Caption1>
          )}
        </div>
        <StatsPanel />
      </aside>
    </section>
  );
};
