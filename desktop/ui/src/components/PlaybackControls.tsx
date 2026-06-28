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
  const setActiveView = useStore((s) => s.setActiveView);
  const jumpTo = useStore((s) => s.jumpTo);
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
  const isPaused = mode === "paused";
  const hasBoard = session.width > 0 && session.height > 0;
  const canStep = isPaused && !session.completed;
  const canRestart = !inSetup;
  // Edit hops back to the Edit pane (and cancels playback first). It only
  // makes sense from a quiescent state; pausing a live run is the user's
  // explicit call, so we don't sneak it in here.
  const canEdit = (inSetup || isPaused) && hasBoard;
  // One button instead of two — "Play" in setup means "Start the run",
  // "Play" while paused means "Resume", and the same button flips to
  // "Pause" while a run is in flight.
  const showPause = isPlaying || isJumping;
  const canPlay = !showPause && hasBoard && !session.completed;
  const canPause = isPlaying || isJumping;
  const canJump = isPaused;
  const playOrPause = () => {
    if (showPause) {
      void pause();
      return;
    }
    if (inSetup) {
      void startRun();
      return;
    }
    void play(gps);
  };

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

      {showPause ? (
        <Tooltip content="Pause (Space)" relationship="label">
          <ToolbarButton
            appearance="primary"
            icon={<PauseRegular />}
            disabled={!canPause}
            onClick={playOrPause}
          >
            Pause
          </ToolbarButton>
        </Tooltip>
      ) : (
        <Tooltip
          content={inSetup ? "Start the simulation (Space)" : "Play (Space)"}
          relationship="label"
        >
          <ToolbarButton
            appearance="primary"
            icon={<PlayRegular />}
            disabled={!canPlay}
            onClick={playOrPause}
          >
            Play
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

      <Tooltip
        content={
          canEdit
            ? "Return the board to setup and open the Edit pane (Esc)"
            : "Pause the run first, then you can edit the board"
        }
        relationship="label"
      >
        <ToolbarButton
          icon={<CursorRegular />}
          disabled={!canEdit}
          onClick={() => {
            void (async () => {
              await editBoard();
              setActiveView("edit");
            })();
          }}
        >
          Edit Board
        </ToolbarButton>
      </Tooltip>

      <ToolbarDivider />
      <Field label={`Speed ${gps} gps`} className={styles.speedSlider}>
        <Slider
          min={MIN_GPS}
          max={MAX_GPS}
          value={gps}
          onChange={(_, data) => setGps(data.value)}
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
