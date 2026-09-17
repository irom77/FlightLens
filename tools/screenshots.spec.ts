import { aiFixture } from "./llmFixture";
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
        invoke: async (
          command: string,
          args: { offset?: number; count?: number },
        ) => {
          if (command === "pending_sources" || command === "choose_files")
            return [];
          if (command === "restore_session")
            return { sources: [], unavailable: [] };
          if (command === "ingest_text") return f.fixture.artifact;
          if (command === "raw_page")
            return f.fixture.rawSyntax.slice(
              args.offset,
              (args.offset ?? 0) + (args.count ?? 500),
            );
          if (command === "inspect_config") return f.fixture.inspection;
          if (command === "llm_settings")
            return {
              settings: {
                enabled: false,
                provider: "gemini",
                model: "gemini-3.8-flash",
                baseUrl: null,
                timeoutSeconds: 60,
              },
              hasKey: false,
              keyHint: null,
              sessionOnly: false,
              credentialProblem: null,
            };
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
      if (tab === "Raw")
        await expect(page.locator(".raw-source")).toHaveAttribute(
          "aria-busy",
          "false",
        );
      await capture(page, `${tab.toLowerCase()}-${theme}`);
    }
  });
}

for (const theme of ["dark", "light"] as const) {
  test(`${theme} AI settings, preview and result`, async ({ page }) => {
    await aiFixture(page, theme);
    await page
      .getByRole("button", { name: "AI summary settings", exact: true })
      .click();
    await expect(
      page.getByRole("dialog", { name: "AI summary settings" }),
    ).toBeVisible();
    await expect(page).toHaveScreenshot(`ai-settings-${theme}.png`);
    await page.getByRole("button", { name: "Done", exact: true }).click();
    await page.getByRole("button", { name: "AI summary", exact: true }).click();
    await expect(
      page.getByRole("dialog", { name: "Review AI request" }),
    ).toBeVisible();
    await expect(page).toHaveScreenshot(`ai-preview-${theme}.png`);
    await page.getByRole("button", { name: "Send to" }).click();
    await expect(page.locator(".ai-result")).toBeVisible();
    await capture(page, `ai-result-${theme}`);
  });
}

for (const theme of ["dark", "light"] as const) {
  test(`${theme} AI comparison preview and result`, async ({ page }) => {
    await aiFixture(page, theme, true, true);
    await expect(page.locator(".document")).toHaveCount(3);
    await page.getByRole("button", { name: "AI summary", exact: true }).click();
    await expect(
      page.getByRole("dialog", { name: "Review AI request" }),
    ).toBeVisible();
    await expect(page).toHaveScreenshot(`ai-compare-preview-${theme}.png`);
    await page.getByRole("button", { name: "Send to" }).click();
    await expect(page.locator(".ai-result")).toBeVisible();
    await capture(page, `ai-compare-result-${theme}`);
  });
}
