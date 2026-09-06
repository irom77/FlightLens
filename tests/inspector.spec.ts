import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
const fixture = JSON.parse(
  execFileSync(
    process.env.FLIGHTLENS_CARGO ?? "/home/irom/.cargo/bin/cargo",
    ["run", "--quiet", "-p", "flightlens-core", "--bin", "preview_fixture"],
    { encoding: "utf8" },
  ),
);

test("offline renderer covers all views with the actual Rust DTO fixture", async ({
  page,
}) => {
  const errors: string[] = [];
  const external: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.route("**/*", (route) => {
    if (new URL(route.request().url()).hostname !== "127.0.0.1") {
      external.push(route.request().url());
      return route.abort();
    }
    return route.continue();
  });
  await page.addInitScript((f) => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (command: string) => {
        if (command === "pending_sources" || command === "choose_files")
          return [];
        if (command === "ingest_text") return f.artifact;
        if (command === "inspect_config") return f.inspection;
        if (command === "export_snippet") return f.snippet;
        return 1;
      },
    };
  }, fixture);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic example" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Rate curves" }),
  ).toBeVisible();
  await expect(page.getByRole("img", { name: /Rate curves/ })).toBeVisible();
  await page.screenshot({ path: "test-results/rates.png", fullPage: true });
  for (const tab of [
    "PID",
    "Filters",
    "Ports",
    "Modes",
    "OSD",
    "Raw",
    "Audit",
    "Export",
  ]) {
    await page.getByRole("tab", { name: tab, exact: tab !== "Audit" }).click();
    await expect(page.getByRole("tabpanel", { name: tab })).toBeVisible();
  }
  await page.getByRole("button", { name: "Validate & preview" }).click();
  await expect(page.locator(".snippet")).toContainText(
    "set rates_type = ACTUAL",
  );
  await page.getByLabel("Include device “save” command").check();
  await expect(page.locator(".snippet")).toHaveCount(0);
  await page.getByRole("tab", { name: "Rates", exact: true }).click();
  await page.getByRole("button", { name: "Line 39", exact: true }).click();
  await expect(page.locator("#line-39")).toHaveClass("highlight-line");
  expect(errors).toEqual([]);
  expect(external).toEqual([]);
});

test("app-focused paste shortcut opens and dismisses the dialog", async ({
  page,
}) => {
  await page.goto("/");
  await page.keyboard.press("Control+Shift+V");
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
