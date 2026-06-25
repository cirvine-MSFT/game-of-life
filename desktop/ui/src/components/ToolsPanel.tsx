import { useEffect, useState } from "react";
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
  EditRegular,
  FolderOpenRegular,
  PanelRightContractRegular,
  PanelRightExpandRegular,
  SaveRegular,
  SettingsRegular,
} from "@fluentui/react-icons";

import { useStore, type ThemeChoice } from "../state/store";
import { SetupPanel } from "./SetupPanel";
import { StatsPanel } from "./StatsPanel";

type ToolsTab = "setup" | "statistics" | "files" | "copilot" | "settings";

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
  actionRow: {
    display: "flex",
    gap: tokens.spacingHorizontalS,
    flexWrap: "wrap",
  },
  metadata: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalXS,
    color: tokens.colorNeutralForeground2,
    overflowWrap: "anywhere",
  },
});

const isToolsTab = (value: unknown): value is ToolsTab =>
  value === "setup" ||
  value === "statistics" ||
  value === "files" ||
  value === "copilot" ||
  value === "settings";

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

const FilesPanel = () => {
  const styles = useStyles();
  const session = useStore((s) => s.session);
  const loadBoardSnapshot = useStore((s) => s.loadBoardSnapshot);
  const loadRunBoard = useStore((s) => s.loadRunBoard);
  const saveBoardSnapshot = useStore((s) => s.saveBoardSnapshot);

  return (
    <section className={styles.placeholder} aria-label="Files panel">
      <Subtitle2>Files</Subtitle2>
      <Body1>
        Load and save editable .gol board snapshots, or import the initial/final board from
        a saved run. Loaded files restore a board into Setup mode at iteration 0 so it can be
        replayed or adjusted.
      </Body1>
      <div className={styles.actionRow}>
        <Button icon={<FolderOpenRegular />} onClick={() => void loadBoardSnapshot()}>
          Load board snapshot
        </Button>
        <Button icon={<FolderOpenRegular />} onClick={() => void loadRunBoard("initial")}>
          Load run initial
        </Button>
        <Button icon={<FolderOpenRegular />} onClick={() => void loadRunBoard("final")}>
          Load run final
        </Button>
        <Button
          icon={<SaveRegular />}
          disabled={!session || session.width === 0}
          onClick={() => void saveBoardSnapshot()}
        >
          Save board snapshot
        </Button>
      </div>
      <Divider />
      <div className={styles.metadata} aria-label="Current board file status">
        <Caption1>
          Board: {session && session.width > 0 ? `${session.width}x${session.height}` : "None"}
        </Caption1>
        <Caption1>Iteration: {session ? session.iteration : 0}</Caption1>
        <Caption1>{session?.dirty ? "Unsaved changes" : "No unsaved changes"}</Caption1>
        <Caption1>{session?.savePath ? `Path: ${session.savePath}` : "Path: Unsaved"}</Caption1>
      </div>
    </section>
  );
};

const PanelContent = ({ selectedTab }: { selectedTab: ToolsTab }) => {
  switch (selectedTab) {
    case "setup":
      return <SetupPanel />;
    case "files":
      return <FilesPanel />;
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
  const mode = useStore((s) => s.session?.mode);
  // Default to the Setup tab while in setup mode so users land directly on
  // the New Run / pattern controls. We only auto-switch on the first
  // mode-resolved render so a user who navigates away (e.g. to Statistics)
  // isn't yanked back every time mode flips.
  const [selectedTab, setSelectedTab] = useState<ToolsTab>("setup");
  const [autoTabApplied, setAutoTabApplied] = useState(false);
  useEffect(() => {
    if (autoTabApplied || mode === undefined) {
      return;
    }
    setSelectedTab(mode === "setup" ? "setup" : "statistics");
    setAutoTabApplied(true);
  }, [autoTabApplied, mode]);

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
        <Tab value="setup" icon={<EditRegular />}>
          Setup
        </Tab>
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
