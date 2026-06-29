import type { ReactElement } from "react";
import {
  Caption1,
  Tab,
  TabList,
  Tooltip,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import type { TabValue } from "@fluentui/react-components";
import {
  ChartMultipleRegular,
  DataTrendingRegular,
  EditRegular,
  PlayRegular,
  SettingsRegular,
} from "@fluentui/react-icons";

import { useStore, type ActiveView } from "../state/store";

const useStyles = makeStyles({
  rail: {
    display: "flex",
    flexDirection: "column",
    width: "64px",
    minWidth: "64px",
    backgroundColor: tokens.colorNeutralBackground1,
    borderRight: `1px solid ${tokens.colorNeutralStroke2}`,
    padding: `${tokens.spacingVerticalM} 0`,
    gap: tokens.spacingVerticalS,
    alignItems: "center",
  },
  tabList: {
    flexGrow: 1,
  },
  brand: {
    writingMode: "vertical-rl",
    transform: "rotate(180deg)",
    color: tokens.colorNeutralForeground3,
    letterSpacing: "0.08em",
    textTransform: "uppercase",
  },
});

interface NavEntry {
  value: Exclude<ActiveView, never>;
  label: string;
  icon: ReactElement;
  disabled?: boolean;
  disabledTooltip?: string;
}

const ENTRIES: NavEntry[] = [
  { value: "edit", label: "Edit board", icon: <EditRegular /> },
  { value: "run", label: "Run", icon: <PlayRegular /> },
  {
    value: "aggregate",
    label: "Aggregate statistics",
    icon: <ChartMultipleRegular />,
  },
  { value: "settings", label: "Settings", icon: <SettingsRegular /> },
  {
    value: "telemetry",
    label: "Telemetry",
    icon: <DataTrendingRegular />,
    disabled: true,
    disabledTooltip:
      "Coming soon — aggregated telemetry across CLI and client runs.",
  },
];

const isActiveView = (value: TabValue): value is ActiveView =>
  value === "edit" ||
  value === "run" ||
  value === "aggregate" ||
  value === "settings" ||
  value === "telemetry";

export const NavRail = () => {
  const styles = useStyles();
  const activeView = useStore((s) => s.activeView);
  const setActiveView = useStore((s) => s.setActiveView);

  return (
    <nav className={styles.rail} aria-label="Primary navigation">
      <TabList
        vertical
        appearance="subtle"
        selectedValue={activeView}
        className={styles.tabList}
        onTabSelect={(_, data) => {
          if (!isActiveView(data.value)) return;
          // Disabled tabs are not selectable, but defend against the type
          // narrowing escape hatch.
          const entry = ENTRIES.find((e) => e.value === data.value);
          if (!entry || entry.disabled) return;
          setActiveView(data.value);
        }}
      >
        {ENTRIES.map((entry) =>
          entry.disabled ? (
            <Tooltip
              key={entry.value}
              content={entry.disabledTooltip ?? "Coming soon"}
              relationship="description"
            >
              <Tab
                value={entry.value}
                icon={entry.icon}
                disabled
                aria-label={entry.label}
                aria-disabled
              />
            </Tooltip>
          ) : (
            <Tooltip
              key={entry.value}
              content={entry.label}
              relationship="label"
            >
              <Tab
                value={entry.value}
                icon={entry.icon}
                aria-label={entry.label}
              />
            </Tooltip>
          ),
        )}
      </TabList>
      <Caption1 className={styles.brand}>Game of Life</Caption1>
    </nav>
  );
};
