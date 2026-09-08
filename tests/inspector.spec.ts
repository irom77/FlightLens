import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
const fixture = JSON.parse(
  execFileSync(
    process.env.FLIGHTLENS_CARGO ?? "cargo",
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
      invoke: async (
        command: string,
        args: { configId?: string; rateProfile?: number },
      ) => {
        if (command === "pending_sources" || command === "choose_files")
          return [];
        if (command === "restore_session")
          return { sources: [], unavailable: [] };
        if (command === "ingest_text") return f.artifact;
        if (command === "inspect_config") {
          if (
            args.configId !== f.artifact.document.id ||
            args.rateProfile === undefined ||
            !(args.rateProfile in f.inspections)
          )
            throw new Error("Unexpected inspection request");
          return f.inspections[args.rateProfile];
        }
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
  // The snapshot header names the craft the backup declares alongside the
  // filename and the firmware badge.
  await expect(page.locator(".document-heading .metadata")).toContainText(
    "Craft FlightLens example",
  );
  // The firmware badge carries the flashed target the backup names.
  await expect(page.locator(".document-heading .firmware")).toHaveText(
    "betaflight 4.5.0 · SYNTHETIC",
  );
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
  await expect(
    page.getByRole("region", { name: "Expected backup format" }),
  ).toContainText("dump all");
  await page.keyboard.press("Control+Shift+V");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page
      .getByRole("dialog")
      .getByRole("region", { name: "Expected backup format" }),
  ).toContainText("No reset");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

for (const [skipEmpty, firmware42] of [
  [false, false],
  [true, false],
  [false, true],
]) {
  test(`profile changes preserve partial data; skip empty: ${skipEmpty}; BF4.2: ${firmware42}`, async ({
    page,
  }) => {
    const f = JSON.parse(
      execFileSync(
        process.env.FLIGHTLENS_CARGO ?? "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "flightlens-core",
          "--bin",
          "preview_fixture",
          "--",
          "--explicit-only",
          ...(skipEmpty ? ["--empty-profiles"] : []),
          ...(firmware42 ? ["--betaflight-42"] : []),
        ],
        { encoding: "utf8" },
      ),
    );
    await page.addInitScript((fixture) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (
          command: string,
          args: { configId?: string; rateProfile?: number },
        ) => {
          if (command === "pending_sources" || command === "choose_files")
            return [];
          if (command === "ingest_text") return fixture.artifact;
          if (command === "inspect_config") {
            if (
              args.configId !== fixture.artifact.document.id ||
              args.rateProfile === undefined ||
              !(args.rateProfile in fixture.inspections)
            )
              throw new Error("Unexpected inspection request");
            return fixture.inspections[args.rateProfile];
          }
          return 1;
        },
      };
    }, f);
    await page.goto("/");
    await page
      .getByRole("button", { name: "Explore a synthetic example" })
      .click();
    if (firmware42) {
      await expect(page.locator(".firmware")).toContainText("4.2.11");
      await expect(
        page.getByText("This firmware version is not supported", {
          exact: false,
        }),
      ).toHaveCount(0);
    }
    const populated = skipEmpty ? 3 : 1;
    const partial = skipEmpty ? 4 : 2;
    await expect(
      page.getByRole("combobox", { name: "Rate profile" }),
    ).toHaveValue(String(populated - 1));
    await expect(
      page.getByRole("combobox", { name: "Rate profile" }).locator("option"),
    ).toHaveText([
      `${populated} · CLI ${populated - 1}`,
      `${partial} · CLI ${partial - 1}`,
    ]);
    await expect(page.locator(".profile-card")).toHaveCount(2);
    const plot = page.getByRole("img", { name: /Rate curves/ });
    await expect(plot.locator("path")).toHaveCount(3);
    await expect(page.locator(".stats .stat strong")).toHaveText([
      "800 °/s",
      "800 °/s",
      "650 °/s",
    ]);
    await page
      .getByRole("button", {
        name: new RegExp(`Profile ${partial}.*explicit settings`),
      })
      .click();
    await expect(plot.locator("path")).toHaveCount(1);
    await expect(page.locator(".stats .stat strong")).toHaveText([
      "667 °/s",
      "Unavailable",
      "Unavailable",
    ]);
    await page
      .getByRole("button", {
        name: new RegExp(`Profile ${populated}.*explicit settings`),
      })
      .click();
    await expect(plot.locator("path")).toHaveCount(3);
    await page.getByRole("tab", { name: "PID", exact: true }).click();
    await expect(
      page.getByRole("combobox", { name: "PID profile" }),
    ).toHaveValue(String(populated - 1));
    await expect(
      page.getByRole("combobox", { name: "PID profile" }).locator("option"),
    ).toHaveText([
      `${populated} · CLI ${populated - 1}`,
      `${partial} · CLI ${partial - 1}`,
    ]);
    await expect(page.locator(".profile-card")).toHaveCount(2);
    const rows = page
      .getByRole("tabpanel", { name: "PID" })
      .locator("table")
      .first()
      .locator("tbody tr");
    for (const [index, expected] of [
      ["45", "80", "30", "120"],
      ["47", "84", "34", "125"],
      ["45", "80", "0", "120"],
    ].entries()) {
      for (const [column, value] of expected.entries()) {
        await expect(rows.nth(index).locator("td").nth(column)).toHaveText(
          value,
        );
      }
    }
    await page
      .getByRole("button", {
        name: new RegExp(`Profile ${partial}.*explicit settings`),
      })
      .click();
    for (let index = 0; index < 3; index++) {
      for (let column = 0; column < 4; column++) {
        await expect(rows.nth(index).locator("td").nth(column)).toHaveText(
          "Unknown",
        );
      }
    }
  });
}

for (const scenario of ["--vendor-missing-expo", "--zero-expo"]) {
  test(`Expo evidence: ${scenario}`, async ({ page }) => {
    const f = JSON.parse(
      execFileSync(
        process.env.FLIGHTLENS_CARGO ?? "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "flightlens-core",
          "--bin",
          "preview_fixture",
          "--",
          scenario,
        ],
        { encoding: "utf8" },
      ),
    );
    await page.addInitScript((fixture) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (
          command: string,
          args: { configId?: string; rateProfile?: number },
        ) => {
          if (command === "pending_sources" || command === "choose_files")
            return [];
          if (command === "ingest_text") return fixture.artifact;
          if (command === "inspect_config") {
            if (
              args.configId !== fixture.artifact.document.id ||
              args.rateProfile === undefined ||
              !(args.rateProfile in fixture.inspections)
            )
              throw new Error("Unexpected inspection request");
            return fixture.inspections[args.rateProfile];
          }
          return 1;
        },
      };
    }, f);
    await page.goto("/");
    await page
      .getByRole("button", { name: "Explore a synthetic example" })
      .click();
    const missing = scenario === "--vendor-missing-expo";
    const stats = page.locator(".stats .stat");
    await expect(stats).toHaveCount(3);
    for (let axis = 0; axis < 3; axis++) {
      await expect(stats.nth(axis)).toContainText(
        missing ? "Expo unknown" : "Expo 0",
      );
    }
    await expect(
      page.getByRole("img", { name: /Rate curves/ }).locator("path"),
    ).toHaveCount(missing ? 0 : 3);
    if (missing) {
      await expect(
        page.getByText("4.5.3.KAACK_V19", { exact: false }).first(),
      ).toBeVisible();
      await expect(
        page.getByText(/For complete inspection, capture/),
      ).toContainText("dump all");
    } else {
      await expect(page.locator(".stats")).not.toContainText("Unavailable");
    }
  });
}

for (const fullDump of [false, true]) {
  test(`backup warning remains visible after file import; dump all: ${fullDump}`, async ({ page }) => {
    const f = structuredClone(fixture);
    if (fullDump) f.artifact.document.syntax.push({ raw: "# dump all" });
    await page.addInitScript((data) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (command: string) => {
          if (command === "pending_sources") return [];
          if (command === "choose_files") return [{ id: "synthetic", label: "Synthetic backup" }];
          if (command === "open_source") return data.artifact;
          if (command === "inspect_config") return data.inspection;
          return 1;
        },
      };
    }, f);
    await page.goto("/");
    const guidance = page.getByRole("region", { name: "Backup import warning" });
    await expect(guidance).toBeVisible();
    await expect(guidance).toContainText("dump all");
    await page.getByRole("button", { name: "Open backups" }).click();
    await expect(page.getByRole("heading", { name: "Rate curves" })).toBeVisible();
    await expect(guidance).toBeVisible();
    const warning = page.getByRole("alert", { name: "Incomplete backup warning" });
    await expect(warning).toHaveCount(fullDump ? 0 : 1);
    await page.getByRole("tab", { name: "PID", exact: true }).click();
    await expect(warning).toHaveCount(fullDump ? 0 : 1);
  });
}

test("theme switch applies immediately and is remembered", async ({ page }) => {
  await page.goto("/");
  const root = page.locator("html");
  const chrome = page.locator('meta[name="theme-color"]');
  const started = await root.getAttribute("data-theme");
  const chromeBefore = await chrome.getAttribute("content");
  expect(started === "dark" || started === "light").toBe(true);
  const switched = started === "dark" ? "light" : "dark";
  await page
    .getByRole("button", { name: new RegExp(`Switch to ${switched} mode`) })
    .click();
  await expect(root).toHaveAttribute("data-theme", switched);
  await expect(chrome).not.toHaveAttribute("content", chromeBefore ?? "");
  await expect(
    page.getByRole("button", {
      name: new RegExp(`Switch to ${started} mode`),
    }),
  ).toBeVisible();
  await page.reload();
  await expect(root).toHaveAttribute("data-theme", switched);
});
