import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed dev-server port so its native-side hot-reload can
// target a predictable URL. Strictly fail rather than fall back to a random
// port so dev mode never silently drifts away from tauri.conf.json.
// Using `vitest/config` instead of `vite` so we can co-locate the `test`
// block; Vitest 3 ships a Vite 6-compatible `defineConfig`.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: true,
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    coverage: {
      reporter: ["text", "html"],
    },
  },
});
