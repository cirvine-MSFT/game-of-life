import { useState } from "react";
import {
  Body1,
  Button,
  Caption1,
  Divider,
  Radio,
  RadioGroup,
  Subtitle2,
  Tab,
  TabList,
  Tooltip,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  BotRegular,
  ChartMultipleRegular,
  FolderOpenRegular,
  PanelRightContractRegular,
  PanelRightExpandRegular,
  SettingsRegular,
} from "@fluentui/react-icons";

import { useStore, type ThemeChoice } from "../state/store";
import { StatsPanel } from "./StatsPanel";

type ToolsTab = "statistics" | "files" | "copilot" | "settings";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    backgroundColor: tokens.colorNeutralBackground1,
    borderLeft: `1px solid ${tokens.colorNeutralStroke2}`,
    color: tokens.colorNeutralForeground1,
  },
  expanded: {
    width: "320px",
    minWidth: "320px",
  },
  collapsed: {
    width: "56px",
    minWidth: "56px",
    alignItems: "center",
    gap: tokens.spacingVerticalM,
    padding: `${tokens.spacingVerticalM} ${tokens.spacingHorizontalXS}`,
  },
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: tokens.spacingHorizontalS,
    padding: `${tokens.spacingVerticalS} ${tokens.spacingHorizontalM}`,
    borderBottom: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  title: {
    display: "flex",
    alignItems: "center",
    gap: tokens.spacingHorizontalS,
  },
  tabs: {
    padding: `${tokens.spacingVerticalS} ${tokens.spacingHorizontalM}`,
    borderBottom: `1px solid ${tokens.colorNeutralStroke2}`,
  },
  content: {
    minHeight: 0,
    overflowY: "auto",
    padding: tokens.spacingHorizontalM,
  },
  railLabel: {
    writingMode: "vertical-rl",
    transform: "rotate(180deg)",
    letterSpacing: "0.08em",
    textTransform: "uppercase",
    color: tokens.colorNeutralForeground3,
  },
  placeholder: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
  },
  themeOptions: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
  },
});

const isToolsTab = (value: unknown): value is ToolsTab =>
  value === "statistics" || value === "files" || value === "copilot" || value === "settings";

const isThemeChoice = (value: string): value is ThemeChoice =>
  value === "light" || value === "dark" || value === "highContrast" || value === "system";

const PlaceholderSection = ({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) => {
  const styles = useStyles();
  return (
    <section className={styles.placeholder} aria-label={`${title} panel`}>
      <Subtitle2>{title}</Subtitle2>
      <Body1>{children}</Body1>
    </section>
  );
};

const SettingsPanel = () => {
  const styles = useStyles();
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  return (
    <section className={styles.placeholder} aria-label="Settings panel">
      <Subtitle2>Settings</Subtitle2>
      <Body1>Choose the application theme without leaving the tools panel.</Body1>
      <Divider />
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
    </section>
  );
};

const PanelContent = ({ selectedTab }: { selectedTab: ToolsTab }) => {
  switch (selectedTab) {
    case "files":
      return (
        <PlaceholderSection title="Files">
          Load, save, and pattern-library actions will live here. The current board snapshot save action
          remains available from the playback toolbar while this navigation surface is established.
        </PlaceholderSection>
      );
    case "copilot":
      return (
        <PlaceholderSection title="Copilot">
          Future Copilot SDK prompts, generated pattern ideas, and AI-assisted board commands have a
          dedicated home in this panel.
        </PlaceholderSection>
      );
    case "settings":
      return <SettingsPanel />;
    case "statistics":
    default:
      return <StatsPanel />;
  }
};

export const ToolsPanel = () => {
  const styles = useStyles();
  const [collapsed, setCollapsed] = useState(false);
  const [selectedTab, setSelectedTab] = useState<ToolsTab>("statistics");

  if (collapsed) {
    return (
      <aside className={`${styles.root} ${styles.collapsed}`} aria-label="Tools panel">
        <Tooltip content="Open tools panel" relationship="label">
          <Button
            appearance="subtle"
            icon={<PanelRightExpandRegular />}
            aria-label="Open tools panel"
            onClick={() => setCollapsed(false)}
          />
        </Tooltip>
        <Caption1 className={styles.railLabel}>Tools</Caption1>
      </aside>
    );
  }

  return (
    <aside className={`${styles.root} ${styles.expanded}`} aria-label="Tools panel">
      <div className={styles.header}>
        <div className={styles.title}>
          <PanelRightContractRegular aria-hidden />
          <Subtitle2>Tools panel</Subtitle2>
        </div>
        <Tooltip content="Close tools panel" relationship="label">
          <Button
            appearance="subtle"
            icon={<PanelRightContractRegular />}
            aria-label="Close tools panel"
            onClick={() => setCollapsed(true)}
          />
        </Tooltip>
      </div>

      <TabList
        className={styles.tabs}
        selectedValue={selectedTab}
        onTabSelect={(_, data) => {
          if (isToolsTab(data.value)) {
            setSelectedTab(data.value);
          }
        }}
      >
        <Tab value="statistics" icon={<ChartMultipleRegular />}>
          Statistics
        </Tab>
        <Tab value="files" icon={<FolderOpenRegular />}>
          Files
        </Tab>
        <Tab value="copilot" icon={<BotRegular />}>
          Copilot
        </Tab>
        <Tab value="settings" icon={<SettingsRegular />}>
          Settings
        </Tab>
      </TabList>

      <div className={styles.content}>
        <PanelContent selectedTab={selectedTab} />
      </div>
    </aside>
  );
};
