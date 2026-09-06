import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  workers: 1,
  timeout: 30000,
  use: {
    baseURL: "http://127.0.0.1:1420",
    viewport: { width: 1280, height: 900 },
    launchOptions: process.env.FLIGHTLENS_CHROMIUM
      ? { executablePath: process.env.FLIGHTLENS_CHROMIUM }
      : {},
  },
  webServer: {
    command: "pnpm dev",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: false,
  },
});
