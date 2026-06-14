import { Title3, Body1, makeStyles, tokens } from "@fluentui/react-components";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    height: "100%",
    gap: tokens.spacingVerticalM,
    backgroundColor: tokens.colorNeutralBackground2,
    color: tokens.colorNeutralForeground1,
  },
});

// Scaffold placeholder. The real layout (menu bar, toolbar, BoardCanvas,
// StatsPanel, status bar) lands in the UI-surface todos. This keeps the
// scaffold compileable and visually verifies Fluent UI is wired up.
export const Shell = () => {
  const styles = useStyles();
  return (
    <div className={styles.root}>
      <Title3>Game of Life</Title3>
      <Body1>Desktop visualizer — scaffold in place.</Body1>
    </div>
  );
};
