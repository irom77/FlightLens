import { defineConfig } from "@playwright/test";
// Separate from playwright.config.ts: the README images are an on-demand task,
// not part of the renderer behavior suite. The committed PNGs in
// docs/screenshots are the snapshot baselines, so `pnpm screenshots` refreshes
// them and `pnpm screenshots:check` reports when they have gone stale.
export default defineConfig({
  testDir: "./tools",
  workers: 1,
  timeout: 60000,
  snapshotPathTemplate: "docs/screenshots/{arg}{ext}",
  expect: {
    // Tight enough to catch a restyled control, loose enough to survive small
    // font-rasterization differences between machines.
    toHaveScreenshot: { maxDiffPixelRatio: 0.001, animations: "disabled" },
  },
  use: {
    baseURL: "http://127.0.0.1:1420",
    viewport: { width: 1280, height: 820 },
    deviceScaleFactor: 2,
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
