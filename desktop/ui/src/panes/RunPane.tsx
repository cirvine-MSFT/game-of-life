import { useEffect, useState } from "react";
import {
  Badge,
  Button,
  Caption1,
  Field,
  Input,
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
  maxIterField: {
    display: "flex",
    flexDirection: "column",
    minWidth: "140px",
    gap: tokens.spacingVerticalXS,
  },
  maxIterRow: {
    display: "flex",
    alignItems: "center",
    gap: tokens.spacingHorizontalXS,
  },
  maxIterInput: { width: "92px" },
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
  const extendMaxIterations = useStore((s) => s.extendMaxIterations);

  const sessionMaxIter = session?.maxIterations ?? 0;
  const sessionIter = session?.iteration ?? 0;
  const [maxIterInput, setMaxIterInput] = useState(() => String(sessionMaxIter));

  // Re-sync the local input whenever the backend's max changes — loading
  // a saved run pulls its max-iterations from the file, and the user
  // shouldn't have to remember to update the field manually.
  useEffect(() => {
    setMaxIterInput(String(sessionMaxIter));
  }, [sessionMaxIter]);

  const parsedMaxIter = (() => {
    const trimmed = maxIterInput.trim();
    if (trimmed === "") return null;
    const n = Number.parseInt(trimmed, 10);
    return Number.isFinite(n) ? n : null;
  })();
  // Backend rejects new_total < current iteration to avoid corrupting
  // the series. Mirror that limit in the UI so Apply doesn't fire and
  // immediately surface an error.
  const minAllowedMax = Math.max(1, sessionIter);
  const canApplyMax =
    session !== null &&
    parsedMaxIter !== null &&
    parsedMaxIter >= minAllowedMax &&
    parsedMaxIter !== sessionMaxIter;

  const applyMaxIter = () => {
    if (!canApplyMax || parsedMaxIter === null) return;
    void extendMaxIterations(parsedMaxIter);
  };

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
          <ToolbarDivider />
          <Field
            className={styles.maxIterField}
            label={`Max iterations${sessionMaxIter ? ` (current ${sessionMaxIter})` : ""}`}
          >
            <div className={styles.maxIterRow}>
              <Input
                className={styles.maxIterInput}
                aria-label="Max iterations"
                type="number"
                min={minAllowedMax}
                value={maxIterInput}
                disabled={!session}
                onChange={(_, data) => setMaxIterInput(data.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    applyMaxIter();
                  }
                }}
              />
              <Button
                appearance="secondary"
                disabled={!canApplyMax}
                onClick={applyMaxIter}
              >
                Apply
              </Button>
            </div>
          </Field>
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
