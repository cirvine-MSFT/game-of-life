import { useState } from "react";
import {
  Body1,
  Button,
  Caption1,
  Divider,
  Dropdown,
  Field,
  Input,
  Option,
  Subtitle2,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  DeleteRegular,
  GridRegular,
  PlayRegular,
  ArrowShuffleRegular,
} from "@fluentui/react-icons";

import { useStore } from "../state/store";
import type { PatternName } from "../ipc";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
  },
  section: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
  },
  actionRow: {
    display: "flex",
    flexWrap: "wrap",
    gap: tokens.spacingHorizontalS,
  },
  inlineRow: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: tokens.spacingHorizontalS,
  },
  metadata: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalXS,
    color: tokens.colorNeutralForeground2,
  },
});

const PATTERNS: { value: PatternName; label: string }[] = [
  { value: "demo", label: "Demo (centered blinker)" },
  { value: "blinker", label: "Blinker" },
  { value: "fullyAlive", label: "Fully alive" },
];

const isPatternName = (value: string): value is PatternName =>
  value === "demo" || value === "blinker" || value === "fullyAlive";

const parseNonNegativeInt = (raw: string): number | null => {
  if (!/^\d+$/.test(raw.trim())) {
    return null;
  }
  const parsed = Number(raw.trim());
  return Number.isFinite(parsed) ? parsed : null;
};

/**
 * Tools-panel tab that owns "set up the current board". Hosts the
 * New Run launcher plus in-place setup actions (clear, apply pattern,
 * randomize) that operate on the existing board allocation. In-place
 * actions are gated on Setup mode because the backend rejects them
 * outside it; the dialog itself works in any mode because create_run
 * tears down the worker on the Rust side.
 */
export const SetupPanel = () => {
  const styles = useStyles();
  const session = useStore((s) => s.session);
  const openNewRunDialog = useStore((s) => s.openNewRunDialog);
  const clearBoard = useStore((s) => s.clearBoard);
  const applyPattern = useStore((s) => s.applyPattern);
  const randomize = useStore((s) => s.randomize);

  const [pattern, setPattern] = useState<PatternName>("demo");
  const [seed, setSeed] = useState("1");
  const [density, setDensity] = useState("300");
  const [actionError, setActionError] = useState<string | null>(null);

  const inSetup = session?.mode === "setup";
  const hasBoard = !!session && session.width > 0;
  const inPlaceDisabled = !inSetup || !hasBoard;

  const runAction = async (op: () => Promise<void>) => {
    setActionError(null);
    try {
      await op();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    }
  };

  const onApplyPattern = () => {
    void runAction(() => applyPattern(pattern));
  };

  const onRandomize = () => {
    const seedValue = parseNonNegativeInt(seed);
    const densityValue = parseNonNegativeInt(density);
    if (seedValue === null || densityValue === null || densityValue > 1000) {
      setActionError(
        "Seed must be a non-negative whole number and density must be between 0 and 1000.",
      );
      return;
    }
    void runAction(() => randomize(seedValue, densityValue));
  };

  return (
    <section className={styles.root} aria-label="Setup panel">
      <Subtitle2>Setup</Subtitle2>
      <Body1>
        Start a new run from scratch, or change the current board in place
        without leaving Setup mode.
      </Body1>

      <Button
        appearance="primary"
        icon={<PlayRegular />}
        onClick={() => openNewRunDialog()}
      >
        New Run…
      </Button>

      <Divider />

      <section className={styles.section} aria-label="In-place setup actions">
        <Subtitle2>Modify current board</Subtitle2>
        {!inSetup && hasBoard && (
          <Caption1>
            Return to Setup mode (Edit Board) to clear, paint, or randomize.
          </Caption1>
        )}

        <div className={styles.actionRow}>
          <Button
            icon={<DeleteRegular />}
            disabled={inPlaceDisabled}
            onClick={() => void runAction(clearBoard)}
          >
            Clear board
          </Button>
        </div>

        <Field label="Apply pattern">
          <div className={styles.actionRow}>
            <Dropdown
              aria-label="Pattern"
              value={PATTERNS.find((p) => p.value === pattern)?.label ?? ""}
              selectedOptions={[pattern]}
              onOptionSelect={(_, data) => {
                if (data.optionValue && isPatternName(data.optionValue)) {
                  setPattern(data.optionValue);
                }
              }}
              disabled={inPlaceDisabled}
            >
              {PATTERNS.map((p) => (
                <Option key={p.value} value={p.value}>
                  {p.label}
                </Option>
              ))}
            </Dropdown>
            <Button
              icon={<GridRegular />}
              disabled={inPlaceDisabled}
              onClick={onApplyPattern}
            >
              Apply
            </Button>
          </div>
        </Field>

        <Field label="Randomize">
          <div className={styles.inlineRow}>
            <Field label="Seed" size="small">
              <Input
                type="number"
                min={0}
                value={seed}
                onChange={(_, data) => setSeed(data.value)}
                disabled={inPlaceDisabled}
              />
            </Field>
            <Field label="Alive per 1000" size="small">
              <Input
                type="number"
                min={0}
                max={1000}
                value={density}
                onChange={(_, data) => setDensity(data.value)}
                disabled={inPlaceDisabled}
              />
            </Field>
          </div>
          <Button
            icon={<ArrowShuffleRegular />}
            disabled={inPlaceDisabled}
            onClick={onRandomize}
            style={{ marginTop: tokens.spacingVerticalS, alignSelf: "flex-start" }}
          >
            Randomize
          </Button>
        </Field>

        {actionError && (
          <Caption1 role="alert" style={{ color: tokens.colorPaletteRedForeground1 }}>
            {actionError}
          </Caption1>
        )}
      </section>

      <Divider />

      <div className={styles.metadata} aria-label="Current board summary">
        <Caption1>
          Board: {hasBoard ? `${session!.width}x${session!.height}` : "None"}
        </Caption1>
        <Caption1>Mode: {session?.mode ?? "—"}</Caption1>
        <Caption1>Max iterations: {session ? session.maxIterations : "—"}</Caption1>
      </div>
    </section>
  );
};
