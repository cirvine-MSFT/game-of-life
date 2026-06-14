import { FluentProvider, webLightTheme } from "@fluentui/react-components";
import { Shell } from "./components/Shell";

// Top-level provider keeps the entire UI tree under Fluent UI's theme tokens.
// `webLightTheme` is the v1 default; theme switching wires in via the
// `theming` todo and a higher-level store later.
export const App = () => (
  <FluentProvider theme={webLightTheme} style={{ height: "100vh" }}>
    <Shell />
  </FluentProvider>
);
