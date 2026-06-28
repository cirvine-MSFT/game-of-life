import {
  Body1,
  Caption1,
  Subtitle2,
  makeStyles,
  tokens,
} from "@fluentui/react-components";

// Placeholder pane bodies. These exist so AppShell builds and the nav rail
// has somewhere to render while the dedicated RunPane / AggregatePane todos
// are in flight. Once each real pane lands it replaces the matching
// placeholder import here.

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalS,
    padding: tokens.spacingHorizontalL,
    alignItems: "flex-start",
  },
});

interface Props {
  name: string;
  body: string;
}

const PanePlaceholder = ({ name, body }: Props) => {
  const styles = useStyles();
  return (
    <section className={styles.root} aria-label={`${name} placeholder`}>
      <Subtitle2>{name}</Subtitle2>
      <Body1>{body}</Body1>
      <Caption1>Placeholder — full pane lands in a follow-up commit.</Caption1>
    </section>
  );
};

export const RunPanePlaceholder = () => (
  <PanePlaceholder
    name="Run"
    body="Load a board or saved run, then watch the simulation play out with full playback controls."
  />
);

export const AggregatePanePlaceholder = () => (
  <PanePlaceholder
    name="Aggregate statistics"
    body="Select multiple .gol run files and compare their per-generation behavior."
  />
);
