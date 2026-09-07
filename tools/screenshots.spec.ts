import { test, expect, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
// README images come from the synthetic example: the same Rust DTO fixture the
// renderer tests use, so the documentation never shows a real backup.
// `pnpm screenshots` rewrites the committed images; `pnpm screenshots:check`
// compares against them and fails when they no longer match the interface.
const fixture = JSON.parse(
  execFileSync(
    process.env.FLIGHTLENS_CARGO ?? "cargo",
    ["run", "--quiet", "-p", "flightlens-core", "--bin", "preview_fixture"],
    { encoding: "utf8" },
  ),
);
const openExample = async (page: Page, theme: "dark" | "light") => {
  await page.addInitScript(
    (f) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (command: string) => {
          if (command === "pending_sources" || command === "choose_files")
            return [];
          if (command === "ingest_text") return f.fixture.artifact;
          if (command === "inspect_config") return f.fixture.inspection;
          if (command === "export_snippet") return f.fixture.snippet;
          return 1;
        },
      };
      localStorage.setItem("flightlens.theme", f.theme);
    },
    { fixture, theme },
  );
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic example" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Rate curves" }),
  ).toBeVisible();
};
// Clicking scrolls the welcome page, so every capture starts at the top bar.
const capture = async (page: Page, name: string) => {
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(page.locator(".topbar")).toBeInViewport();
  await expect(page).toHaveScreenshot(`${name}.png`);
};
for (const theme of ["dark", "light"] as const) {
  test(`${theme} mode screenshots match the README images`, async ({
    page,
  }) => {
    await openExample(page, theme);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await capture(page, `rates-${theme}`);
    for (const tab of ["PID", "Filters", "Raw", "Audit"]) {
      await page
        .getByRole("tab", { name: tab, exact: tab !== "Audit" })
        .click();
      await expect(page.getByRole("tabpanel", { name: tab })).toBeVisible();
      await capture(page, `${tab.toLowerCase()}-${theme}`);
    }
  });
}
