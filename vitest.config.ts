import { defineConfig } from "vitest/config";
import vite from "./vite.config";

export default defineConfig({
  ...vite,
  test: {
    // Playwright owns tests/ (renderer behavior) and tools/ (README screenshots).
    exclude: ["tests/**", "tools/**", "node_modules/**"],
  },
});
