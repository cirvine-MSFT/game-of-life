import { useEffect, useMemo, useState } from "react";
import {
  Body1,
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  Field,
  Input,
  MessageBar,
  MessageBarBody,
  Radio,
  RadioGroup,
  Subtitle2,
  makeStyles,
  tokens,
} from "@fluentui/react-components";

import { useStore } from "../state/store";
import type { CreateRunArgs, InitialSource, PatternName } from "../ipc";

// The desktop budget allows ~16M cells with default settings; capping at
// 2048 in each dimension keeps the dialog within sensible single-machine
// limits and matches the Streaming-Not-Implemented backend guard.
const MAX_DIMENSION = 2048;
const MAX_ITERATIONS_CAP = 1_000_000_000;
const DENSITY_MAX = 1000;

type SourceKind = "demo" | "blinker" | "fullyAlive" | "empty" | "random";

const SOURCE_OPTIONS: { value: SourceKind; label: string }[] = [
  { value: "demo", label: "Demo pattern (centered blinker)" },
  { value: "blinker", label: "Blinker" },
  { value: "fullyAlive", label: "Fully alive" },
  { value: "empty", label: "Empty board" },
  { value: "random", label: "Random" },
];

const useStyles = makeStyles({
  surface: {
    maxWidth: "560px",
  },
  grid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: tokens.spacingHorizontalM,
  },
  column: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
  },
  section: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
  },
  errorBar: {
    marginBottom: tokens.spacingVerticalS,
  },
});

interface FormState {
  width: string;
  height: string;
  maxIterations: string;
  source: SourceKind;
  seed: string;
  density: string;
}

const DEFAULTS: FormState = {
  width: "20",
  height: "20",
  maxIterations: "100",
  source: "demo",
  seed: "1",
  density: "300",
};

const isSourceKind = (value: string): value is SourceKind =>
  value === "demo" ||
  value === "blinker" ||
  value === "fullyAlive" ||
  value === "empty" ||
  value === "random";

const parseInteger = (raw: string): number | null => {
  if (!/^\d+$/.test(raw.trim())) {
    return null;
  }
  const parsed = Number(raw.trim());
  return Number.isFinite(parsed) ? parsed : null;
};

const messageFromUnknown = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return String(error);
};

const buildSource = (
  kind: SourceKind,
  seed: number,
  density: number,
): InitialSource => {
  switch (kind) {
    case "empty":
      return { kind: "empty" };
    case "random":
      return {
        kind: "random",
        value: { seed, aliveCellsPerThousand: density },
      };
    case "demo":
    case "blinker":
    case "fullyAlive": {
      const pattern: PatternName = kind;
      return { kind: "pattern", value: pattern };
    }
  }
};

interface FieldErrors {
  width?: string;
  height?: string;
  maxIterations?: string;
  seed?: string;
  density?: string;
}

const validate = (form: FormState): { args?: CreateRunArgs; errors: FieldErrors } => {
  const errors: FieldErrors = {};
  const width = parseInteger(form.width);
  const height = parseInteger(form.height);
  const maxIterations = parseInteger(form.maxIterations);

  if (width === null || width < 1) {
    errors.width = "Width must be a whole number ≥ 1";
  } else if (width > MAX_DIMENSION) {
    errors.width = `Width must be ≤ ${MAX_DIMENSION}`;
  }
  if (height === null || height < 1) {
    errors.height = "Height must be a whole number ≥ 1";
  } else if (height > MAX_DIMENSION) {
    errors.height = `Height must be ≤ ${MAX_DIMENSION}`;
  }
  if (maxIterations === null || maxIterations < 1) {
    errors.maxIterations = "Max iterations must be a whole number ≥ 1";
  } else if (maxIterations > MAX_ITERATIONS_CAP) {
    errors.maxIterations = `Max iterations must be ≤ ${MAX_ITERATIONS_CAP}`;
  }

  let seed = 0;
  let density = 0;
  if (form.source === "random") {
    const seedParsed = parseInteger(form.seed);
    const densityParsed = parseInteger(form.density);
    if (seedParsed === null) {
      errors.seed = "Seed must be a non-negative whole number";
    } else if (seedParsed > Number.MAX_SAFE_INTEGER) {
      errors.seed = `Seed must be ≤ ${Number.MAX_SAFE_INTEGER}`;
    } else {
      seed = seedParsed;
    }
    if (densityParsed === null || densityParsed > DENSITY_MAX) {
      errors.density = `Density must be between 0 and ${DENSITY_MAX}`;
    } else {
      density = densityParsed;
    }
  }

  if (
    Object.keys(errors).length > 0 ||
    width === null ||
    height === null ||
    maxIterations === null
  ) {
    return { errors };
  }

  return {
    errors,
    args: {
      width,
      height,
      maxIterations,
      source: buildSource(form.source, seed, density),
    },
  };
};

/**
 * Modal that owns "create a fresh board from scratch": board dimensions,
 * iteration budget, and the initial source. Client-side validation runs
 * before we touch the backend; SessionError messages are surfaced inline
 * so the user doesn't lose their form state on the round-trip.
 *
 * The dialog is mounted at the Shell root and driven by the
 * `newRunDialogOpen` flag in the store so any surface (toolbar shortcut,
 * Setup tab) can open it without prop-drilling.
 */
export const NewRunDialog = () => {
  const styles = useStyles();
  const open = useStore((s) => s.newRunDialogOpen);
  const close = useStore((s) => s.closeNewRunDialog);
  const newRun = useStore((s) => s.newRun);
  const session = useStore((s) => s.session);

  const [form, setForm] = useState<FormState>(DEFAULTS);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Reset on every open so the dialog starts from a clean state rather
  // than showing the previous run's data or stale validation errors.
  useEffect(() => {
    if (open) {
      setForm(DEFAULTS);
      setErrors({});
      setSubmitError(null);
      setSubmitting(false);
    }
  }, [open]);

  const isRandom = form.source === "random";

  const update = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  const dirty = useMemo(() => session?.dirty ?? false, [session]);

  const onCreate = async () => {
    const { args, errors: validation } = validate(form);
    setErrors(validation);
    if (!args) {
      return;
    }
    setSubmitError(null);

    if (dirty) {
      const { ask } = await import("@tauri-apps/plugin-dialog");
      const discard = await ask(
        "The current board has unsaved changes. Discard them and create a new run?",
        { title: "Discard unsaved changes?", kind: "warning" },
      );
      if (!discard) {
        return;
      }
    }

    setSubmitting(true);
    try {
      await newRun(args);
      close();
    } catch (error) {
      setSubmitError(messageFromUnknown(error));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(_, data) => {
        if (!data.open) {
          close();
        }
      }}
    >
      <DialogSurface className={styles.surface}>
        <DialogBody>
          <DialogTitle>New Run</DialogTitle>
          <DialogContent>
            {submitError && (
              <MessageBar
                intent="error"
                className={styles.errorBar}
                aria-label="New Run error"
              >
                <MessageBarBody>{submitError}</MessageBarBody>
              </MessageBar>
            )}
            <Body1>
              Configure board dimensions, iteration budget, and how the board
              starts out. The simulation enters Setup mode so you can paint or
              tweak cells before starting the run.
            </Body1>
            <div
              className={styles.grid}
              style={{ marginTop: 12, marginBottom: 12 }}
            >
              <Field
                label="Width"
                required
                validationState={errors.width ? "error" : "none"}
                validationMessage={errors.width}
              >
                <Input
                  type="number"
                  min={1}
                  max={MAX_DIMENSION}
                  value={form.width}
                  onChange={(_, data) => update("width", data.value)}
                />
              </Field>
              <Field
                label="Height"
                required
                validationState={errors.height ? "error" : "none"}
                validationMessage={errors.height}
              >
                <Input
                  type="number"
                  min={1}
                  max={MAX_DIMENSION}
                  value={form.height}
                  onChange={(_, data) => update("height", data.value)}
                />
              </Field>
              <Field
                label="Max iterations"
                required
                validationState={errors.maxIterations ? "error" : "none"}
                validationMessage={errors.maxIterations}
              >
                <Input
                  type="number"
                  min={1}
                  value={form.maxIterations}
                  onChange={(_, data) => update("maxIterations", data.value)}
                />
              </Field>
            </div>
            <section className={styles.section}>
              <Subtitle2>Initial source</Subtitle2>
              <RadioGroup
                aria-label="Initial source"
                value={form.source}
                onChange={(_, data) => {
                  if (isSourceKind(data.value)) {
                    update("source", data.value);
                  }
                }}
              >
                {SOURCE_OPTIONS.map((opt) => (
                  <Radio key={opt.value} value={opt.value} label={opt.label} />
                ))}
              </RadioGroup>
              {isRandom && (
                <div className={styles.grid}>
                  <Field
                    label="Seed"
                    required
                    validationState={errors.seed ? "error" : "none"}
                    validationMessage={errors.seed}
                  >
                    <Input
                      type="number"
                      min={0}
                      value={form.seed}
                      onChange={(_, data) => update("seed", data.value)}
                    />
                  </Field>
                  <Field
                    label="Alive cells per 1000"
                    required
                    validationState={errors.density ? "error" : "none"}
                    validationMessage={errors.density}
                  >
                    <Input
                      type="number"
                      min={0}
                      max={DENSITY_MAX}
                      value={form.density}
                      onChange={(_, data) => update("density", data.value)}
                    />
                  </Field>
                </div>
              )}
            </section>
          </DialogContent>
          <DialogActions>
            <Button appearance="secondary" onClick={() => close()} disabled={submitting}>
              Cancel
            </Button>
            <Button
              appearance="primary"
              onClick={() => void onCreate()}
              disabled={submitting}
            >
              {submitting ? "Creating…" : "Create"}
            </Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
};
