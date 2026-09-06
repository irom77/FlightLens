import { defineConfig } from "vitest/config";
import vite from "./vite.config";

export default defineConfig({
  ...vite,
  test: {
    exclude: ["tests/**", "node_modules/**"],
  },
});
