import {
  Body1,
  Caption1,
  Divider,
  Radio,
  RadioGroup,
  Subtitle2,
  makeStyles,
  tokens,
} from "@fluentui/react-components";

import { ChatPanel } from "../ai/ChatPanel";
import { adjustThemeFromPrompt } from "../ai/aiClient";
import { useStore, type ThemeChoice } from "../state/store";

const useStyles = makeStyles({
  root: {
    display: "grid",
    gridTemplateColumns: "minmax(280px, 1fr) minmax(320px, 1fr)",
    gap: tokens.spacingHorizontalL,
    padding: tokens.spacingHorizontalL,
    height: "100%",
    minHeight: 0,
    "@media (max-width: 960px)": {
      gridTemplateColumns: "1fr",
    },
  },
  column: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
    minHeight: 0,
  },
  card: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
    padding: tokens.spacingHorizontalL,
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: tokens.borderRadiusLarge,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  themeOptions: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
  },
  placeholderNote: {
    color: tokens.colorNeutralForeground3,
  },
});

const isThemeChoice = (value: string): value is ThemeChoice =>
  value === "light" ||
  value === "dark" ||
  value === "highContrast" ||
  value === "system";

export const SettingsPane = () => {
  const styles = useStyles();
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  return (
    <section className={styles.root} aria-label="Settings">
      <div className={styles.column}>
        <div className={styles.card}>
          <Subtitle2>Appearance</Subtitle2>
          <Body1>Theme</Body1>
          <RadioGroup
            aria-label="Theme"
            className={styles.themeOptions}
            value={theme}
            onChange={(_, data) => {
              if (isThemeChoice(data.value)) {
                setTheme(data.value);
              }
            }}
          >
            <Radio value="light" label="Light" />
            <Radio value="dark" label="Dark" />
            <Radio value="highContrast" label="High contrast" />
            <Radio value="system" label="System" />
          </RadioGroup>
        </div>
        <div className={styles.card}>
          <Subtitle2>More settings coming later</Subtitle2>
          <Caption1 className={styles.placeholderNote}>
            Density, hotkeys, and custom palette controls will live here in a
            future release.
          </Caption1>
          <Divider />
          <Caption1 className={styles.placeholderNote}>
            For now, theme is the only app-wide preference.
          </Caption1>
        </div>
      </div>
      <div className={styles.column}>
        <div className={styles.card} style={{ height: "100%" }}>
          <Subtitle2>Ask the AI to tweak the theme</Subtitle2>
          <Caption1 className={styles.placeholderNote}>
            Describe how you want the app to look. Currently a stub — the
            request returns a friendly error until Foundry Local is wired up.
          </Caption1>
          <ChatPanel
            ariaLabel="Theme tweak chat"
            provider={adjustThemeFromPrompt}
            emptyHint="Try: ‘Make it feel like VS Code dark+’ or ‘High-contrast palette with warm accents’."
          />
        </div>
      </div>
    </section>
  );
};
