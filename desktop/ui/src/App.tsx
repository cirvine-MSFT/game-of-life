import {
  FluentProvider,
  webDarkTheme,
  webLightTheme,
  teamsHighContrastTheme,
} from "@fluentui/react-components";

import { Shell } from "./components/Shell";
import { useStore } from "./state/store";

const themeFor = (choice: string) => {
  switch (choice) {
    case "dark":
      return webDarkTheme;
    case "highContrast":
      return teamsHighContrastTheme;
    case "light":
    default:
      return webLightTheme;
  }
};

/**
 * Top-level provider that subscribes to the theme choice in the store
 * so View -> Theme switches apply across the whole tree without
 * remounting components. `system` is treated as `light` for v1 because
 * Tauri v2's media-query detection on Linux is unreliable across
 * desktop environments; auto-follow lands in the theming follow-up.
 */
export const App = () => {
  const theme = useStore((s) => s.theme);
  return (
    <FluentProvider theme={themeFor(theme)} style={{ height: "100vh" }}>
      <Shell />
    </FluentProvider>
  );
};
