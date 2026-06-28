import { useEffect, useState } from "react";
import {
  Body1,
  Button,
  Caption1,
  Input,
  Menu,
  MenuItem,
  MenuList,
  MenuPopover,
  MenuTrigger,
  MessageBar,
  MessageBarActions,
  MessageBarBody,
  Popover,
  PopoverSurface,
  PopoverTrigger,
  Subtitle2,
  Toolbar,
  ToolbarButton,
  ToolbarDivider,
  makeStyles,
  tokens,
} from "@fluentui/react-components";

import { ChatPanel } from "../ai/ChatPanel";
import { generateBoardFromPrompt } from "../ai/aiClient";
import { BoardCanvas } from "../components/BoardCanvas";
import { PATTERN_NAMES, type PatternName } from "../ipc";
import { useStore, type ThemeChoice } from "../state/store";

const MIN_BOARD_SIZE = 1;
const MAX_BOARD_SIZE = 500;
const MIN_ALIVE_PER_THOUSAND = 0;
const MAX_ALIVE_PER_THOUSAND = 1000;

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
  rightColumn: {
    minHeight: 0,
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
  sizeControls: {
    display: "flex",
    alignItems: "center",
    gap: tokens.spacingHorizontalXS,
  },
  sizeInput: {
    width: "84px",
  },
  canvasCard: {
    flex: 1,
    minHeight: "420px",
    overflow: "hidden",
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: tokens.borderRadiusLarge,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  chatCard: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
    height: "100%",
    minHeight: "420px",
    padding: tokens.spacingHorizontalL,
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: tokens.borderRadiusLarge,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  note: {
    color: tokens.colorNeutralForeground3,
  },
  randomForm: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
    minWidth: "260px",
  },
  randomFields: {
    display: "grid",
    gridTemplateColumns: "1fr",
    gap: tokens.spacingVerticalS,
  },
});

const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value));

const parseInteger = (value: string): number | null => {
  if (value.trim() === "") {
    return null;
  }
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : null;
};

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

const patternLabel = (name: PatternName): string =>
  name
    .replace(/([A-Z])/g, " $1")
    .replace(/^./, (first) => first.toUpperCase());

const isActiveRun = (
  session: ReturnType<typeof useStore.getState>["session"],
): boolean => {
  if (!session) {
    return false;
  }
  return (
    session.mode === "playing" ||
    session.mode === "jumpingTo" ||
    (session.mode === "paused" && session.iteration > 0)
  );
};

export const EditPane = () => {
  const styles = useStyles();
  const session = useStore((s) => s.session);
  const theme = useStore((s) => s.theme);
  const clearBoard = useStore((s) => s.clearBoard);
  const randomize = useStore((s) => s.randomize);
  const applyPattern = useStore((s) => s.applyPattern);
  const loadBoardSnapshot = useStore((s) => s.loadBoardSnapshot);
  const saveBoardSnapshot = useStore((s) => s.saveBoardSnapshot);
  const editBoard = useStore((s) => s.editBoard);
  const setActiveView = useStore((s) => s.setActiveView);
  const [widthInput, setWidthInput] = useState(() => String(session?.width ?? ""));
  const [heightInput, setHeightInput] = useState(() => String(session?.height ?? ""));
  const [seedInput, setSeedInput] = useState("0");
  const [aliveInput, setAliveInput] = useState("200");

  useEffect(() => {
    setWidthInput(String(session?.width ?? ""));
    setHeightInput(String(session?.height ?? ""));
  }, [session?.height, session?.width]);

  const mode = session?.mode;
  const inSetup = mode === "setup";
  const hasBoard = (session?.width ?? 0) > 0;
  const showActiveRunCallout = isActiveRun(session);

  const applySize = () => {
    if (!session) {
      return;
    }
    const parsedWidth = parseInteger(widthInput);
    const parsedHeight = parseInteger(heightInput);
    if (parsedWidth === null || parsedHeight === null) {
      return;
    }
    const width = clamp(parsedWidth, MIN_BOARD_SIZE, MAX_BOARD_SIZE);
    const height = clamp(parsedHeight, MIN_BOARD_SIZE, MAX_BOARD_SIZE);
    setWidthInput(String(width));
    setHeightInput(String(height));
    void useStore.getState().newRun({
      width,
      height,
      source: { kind: "empty" },
      maxIterations: session.maxIterations,
    });
  };

  const randomizeFromInputs = () => {
    if (!inSetup) {
      return;
    }
    const seed = parseInteger(seedInput) ?? 0;
    const alive = clamp(
      parseInteger(aliveInput) ?? 200,
      MIN_ALIVE_PER_THOUSAND,
      MAX_ALIVE_PER_THOUSAND,
    );
    setSeedInput(String(seed));
    setAliveInput(String(alive));
    void randomize(seed, alive);
  };

  const applySelectedPattern = (name: PatternName) => {
    if (!inSetup) {
      return;
    }
    void applyPattern(name);
  };

  return (
    <section className={styles.root} aria-label="Edit board">
      <div className={styles.leftColumn}>
        {showActiveRunCallout && (
          <MessageBar intent="warning" role="alert">
            <MessageBarBody>
              A run is in progress. Edit the current board state, or return to setup.
            </MessageBarBody>
            <MessageBarActions>
              <Button onClick={() => setActiveView("run")}>Go to Run</Button>
              <Button onClick={() => void editBoard()}>Return to setup</Button>
            </MessageBarActions>
          </MessageBar>
        )}

        <Toolbar className={styles.toolbar} aria-label="Edit board toolbar">
          <div className={styles.sizeControls}>
            <Input
              className={styles.sizeInput}
              aria-label="Board width"
              type="number"
              min={MIN_BOARD_SIZE}
              max={MAX_BOARD_SIZE}
              value={widthInput}
              onChange={(_, data) => setWidthInput(data.value)}
              disabled={!inSetup}
            />
            <Body1>×</Body1>
            <Input
              className={styles.sizeInput}
              aria-label="Board height"
              type="number"
              min={MIN_BOARD_SIZE}
              max={MAX_BOARD_SIZE}
              value={heightInput}
              onChange={(_, data) => setHeightInput(data.value)}
              disabled={!inSetup}
            />
            <ToolbarButton disabled={!inSetup} onClick={applySize}>
              Apply size
            </ToolbarButton>
          </div>
          <ToolbarDivider />
          <ToolbarButton disabled={!inSetup} onClick={() => void clearBoard()}>
            Clear
          </ToolbarButton>
          <Popover withArrow>
            <PopoverTrigger disableButtonEnhancement>
              <ToolbarButton disabled={!inSetup}>Randomize…</ToolbarButton>
            </PopoverTrigger>
            <PopoverSurface aria-label="Randomize board options">
              <div className={styles.randomForm}>
                <Subtitle2>Random board</Subtitle2>
                <div className={styles.randomFields}>
                  <Input
                    aria-label="Random seed"
                    type="number"
                    value={seedInput}
                    onChange={(_, data) => setSeedInput(data.value)}
                  />
                  <Input
                    aria-label="Alive cells per thousand"
                    type="number"
                    min={MIN_ALIVE_PER_THOUSAND}
                    max={MAX_ALIVE_PER_THOUSAND}
                    value={aliveInput}
                    onChange={(_, data) => setAliveInput(data.value)}
                  />
                </div>
                <Button
                  appearance="primary"
                  disabled={!inSetup}
                  onClick={randomizeFromInputs}
                >
                  Randomize board
                </Button>
              </div>
            </PopoverSurface>
          </Popover>
          <Menu>
            <MenuTrigger disableButtonEnhancement>
              <ToolbarButton disabled={!inSetup}>Pattern</ToolbarButton>
            </MenuTrigger>
            <MenuPopover>
              <MenuList>
                {PATTERN_NAMES.map((name) => (
                  <MenuItem
                    key={name}
                    disabled={!inSetup}
                    onClick={() => applySelectedPattern(name)}
                  >
                    {patternLabel(name)}
                  </MenuItem>
                ))}
              </MenuList>
            </MenuPopover>
          </Menu>
          <ToolbarDivider />
          <ToolbarButton onClick={() => void loadBoardSnapshot()}>
            Load board
          </ToolbarButton>
          <ToolbarButton
            disabled={!hasBoard}
            onClick={() => void saveBoardSnapshot()}
          >
            Save board
          </ToolbarButton>
          <ToolbarDivider />
          <ToolbarButton
            appearance="primary"
            disabled={!hasBoard}
            onClick={() => setActiveView("run")}
          >
            Send to Run ▶
          </ToolbarButton>
        </Toolbar>

        <div className={styles.canvasCard}>
          <BoardCanvas paletteName={paletteNameFor(theme)} readOnly={!inSetup} />
        </div>
      </div>

      <aside className={styles.rightColumn} aria-label="Board generation">
        <div className={styles.chatCard}>
          <Subtitle2>Generate a board with AI</Subtitle2>
          <Caption1 className={styles.note}>
            Describe a starting pattern. This is stubbed until the local AI
            provider is wired up.
          </Caption1>
          <ChatPanel
            ariaLabel="Board generation chat"
            provider={generateBoardFromPrompt}
            emptyHint="Try: 'Glider gun on a 40×40 board' or 'Random spaceships near the top edge'."
          />
        </div>
      </aside>
    </section>
  );
};
