import { useState } from "react";
import {
  Badge,
  Body1,
  Button,
  Field,
  Input,
  Slider,
  Toolbar,
  ToolbarButton,
  ToolbarDivider,
  Tooltip,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  ArrowResetRegular,
  CursorRegular,
  DocumentAddRegular,
  NextRegular,
  PauseRegular,
  PlayRegular,
} from "@fluentui/react-icons";

import { useStore } from "../state/store";
import { formatTerminalStatusFromSession } from "../state/terminalStatus";

const useStyles = makeStyles({
  root: {
    display: "flex",
    alignItems: "center",
    flexWrap: "wrap",
    gap: tokens.spacingHorizontalS,
    padding: tokens.spacingHorizontalS,
    backgroundColor: tokens.colorNeutralBackground1,
    borderBottom: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  spacer: { flexGrow: 1 },
  jumpField: {
    display: "flex",
    alignItems: "center",
    gap: tokens.spacingHorizontalXS,
  },
  jumpInput: { width: "80px" },
  speedSlider: { width: "160px" },
});

const MIN_GPS = 1;
const MAX_GPS = 60;

const modeBadge = (mode: string) => {
  switch (mode) {
    case "setup":
      return { color: "informative" as const, label: "Setup" };
    case "paused":
      return { color: "warning" as const, label: "Paused" };
    case "playing":
      return { color: "success" as const, label: "Playing" };
    case "jumpingTo":
      return { color: "brand" as const, label: "Jumping" };
    default:
      return { color: "subtle" as const, label: mode };
  }
};

/**
 * Toolbar that drives play / pause / step / restart / edit / jump
 * commands. Disables actions that don't make sense for the current mode
 * so the user can't accidentally violate the state-machine invariants
 * the Rust side enforces.
 */
export const PlaybackControls = () => {
  const styles = useStyles();
  const session = useStore((s) => s.session);
  const startRun = useStore((s) => s.startRun);
  const play = useStore((s) => s.play);
  const pause = useStore((s) => s.pause);
  const step = useStore((s) => s.step);
  const restart = useStore((s) => s.restart);
  const editBoard = useStore((s) => s.editBoard);
  const jumpTo = useStore((s) => s.jumpTo);
  const openNewRunDialog = useStore((s) => s.openNewRunDialog);
  const finalStats = useStore((s) => s.finalStats);
  const [gps, setGps] = useState(5);
  const [jumpTarget, setJumpTarget] = useState("");

  if (!session) {
    return null;
  }

  const mode = session.mode;
  const inSetup = mode === "setup";
  const isPlaying = mode === "playing";
  const isJumping = mode === "jumpingTo";
  const canStep = mode === "paused" && !session.completed;
  const canRestart = !inSetup;
  const canEdit = !inSetup;
  const canStart = inSetup;
  const canPlay = mode === "paused" && !session.completed;
  const canPause = isPlaying || isJumping;
  const canJump = mode === "paused";

  const terminal = formatTerminalStatusFromSession(session, finalStats);

  const badge = terminal
    ? { color: terminal.color, label: terminal.label, description: terminal.description }
    : { ...modeBadge(mode), description: undefined as string | undefined };

  const onJump = () => {
    const target = Number.parseInt(jumpTarget, 10);
    if (Number.isFinite(target) && target >= 0) {
      void jumpTo(target);
    }
  };

  return (
    <Toolbar className={styles.root} aria-label="Playback controls">
      <Badge color={badge.color} appearance="filled" aria-label={badge.description ?? badge.label}>
        {badge.label}
      </Badge>
      <Body1>Iteration {session.iteration}</Body1>
      <ToolbarDivider />

      <Tooltip content="Create a new run (board size, iterations, source)" relationship="label">
        <ToolbarButton
          icon={<DocumentAddRegular />}
          onClick={() => openNewRunDialog()}
        >
          New Run…
        </ToolbarButton>
      </Tooltip>

      {canStart && (
        <Tooltip content="Start the simulation (Space)" relationship="label">
          <ToolbarButton appearance="primary" icon={<PlayRegular />} onClick={() => void startRun()}>
            Start
          </ToolbarButton>
        </Tooltip>
      )}

      {!canStart && (
        <>
          {!isPlaying && (
            <Tooltip content="Play (Space)" relationship="label">
              <ToolbarButton
                appearance="primary"
                icon={<PlayRegular />}
                disabled={!canPlay}
                onClick={() => void play(gps)}
              >
                Play
              </ToolbarButton>
            </Tooltip>
          )}
          {isPlaying && (
            <Tooltip content="Pause (Space)" relationship="label">
              <ToolbarButton
                appearance="primary"
                icon={<PauseRegular />}
                disabled={!canPause}
                onClick={() => void pause()}
              >
                Pause
              </ToolbarButton>
            </Tooltip>
          )}

          <Tooltip content="Step one generation (Right arrow)" relationship="label">
            <ToolbarButton icon={<NextRegular />} disabled={!canStep} onClick={() => void step()}>
              Step
            </ToolbarButton>
          </Tooltip>

          <Tooltip content="Restart from initial board (R)" relationship="label">
            <ToolbarButton
              icon={<ArrowResetRegular />}
              disabled={!canRestart}
              onClick={() => void restart()}
            >
              Restart
            </ToolbarButton>
          </Tooltip>

          <Tooltip content="Return to Setup mode (Esc)" relationship="label">
            <ToolbarButton
              icon={<CursorRegular />}
              disabled={!canEdit}
              onClick={() => void editBoard()}
            >
              Edit Board
            </ToolbarButton>
          </Tooltip>
        </>
      )}

      <ToolbarDivider />
      <Field label={`Speed ${gps} gps`} className={styles.speedSlider}>
        <Slider
          min={MIN_GPS}
          max={MAX_GPS}
          value={gps}
          onChange={(_, data) => setGps(data.value)}
          disabled={inSetup}
        />
      </Field>

      <div className={styles.spacer} />

      <div className={styles.jumpField}>
        <Input
          className={styles.jumpInput}
          placeholder="Jump to"
          value={jumpTarget}
          onChange={(_, data) => setJumpTarget(data.value)}
          disabled={!canJump}
          type="number"
          min={0}
        />
        <Button disabled={!canJump} onClick={onJump}>
          Jump
        </Button>
      </div>
    </Toolbar>
  );
};
