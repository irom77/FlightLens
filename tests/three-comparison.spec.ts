import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixtures = [
  [],
  ["--zero-expo"],
  [
    "--feature-differences",
    "--port-differences",
    "--mode-differences",
    "--collection-differences",
    "--rxfail-differences",
    "--adjrange-differences",
    "--vtx-differences",
    "--vtxtable-differences",
  ],
].map((flags) =>
  JSON.parse(
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
        "--throttle-preview",
        "--collection-baseline",
        "--rxfail-baseline",
        "--adjrange-baseline",
        "--vtx-baseline",
        "--vtxtable-baseline",
        ...flags,
      ],
      { encoding: "utf8" },
    ),
  ),
);

test("three-backup overlays, baseline choice, profiles and return to two backups", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (
        command: string,
        args: { sourceId?: string; configId?: string; rateProfile?: number },
      ) => {
        if (command === "restore_session")
          return {
            sources: fixtures.map((f, i) => ({
              id: String(i),
              label: f.artifact.document.title,
            })),
            unavailable: [],
          };
        if (command === "pending_sources") return [];
        if (command === "open_source")
          return fixtures[Number(args.sourceId)].artifact;
        if (command === "inspect_config") {
          const f = fixtures.find(
            (f) => f.artifact.document.id === args.configId,
          );
          return f.inspections[args.rateProfile!];
        }
        return 1;
      },
    };
  }, fixtures);
  await page.goto("/");
  await expect(page.locator(".document")).toHaveCount(3);
  await page
    .getByRole("button", { name: "Compare backups", exact: true })
    .click();
  await page
    .getByLabel("Backup C", { exact: true })
    .selectOption(fixtures[2].artifact.document.id);
  const throttle = page.getByRole("region", {
    name: "Throttle comparison",
    exact: true,
  });
  const roll = page.getByRole("region", {
    name: "roll comparison",
    exact: true,
  });
  await expect(throttle.locator("path")).toHaveCount(3);
  await expect(roll.locator("path")).toHaveCount(3);
  await expect(throttle.locator("path").nth(2)).toHaveAttribute(
    "class",
    "series-2",
  );
  const comparison = page.getByRole("region", {
    name: "Three-backup parameter comparison",
    exact: true,
  });
  await expect(comparison).toContainText("Choose a baseline");
  await page.getByLabel("Comparison baseline").selectOption("0");
  const ranges = page.getByRole("region", {
    name: "Three-backup receiver-range comparison",
    exact: true,
  });
  await expect(ranges.getByRole("row").nth(1)).toContainText("C only");
  await ranges.getByText(/^C: Line/).click();
  await expect(ranges.locator("pre").last()).toHaveText("rxrange 0 1050 1950");
  for (const name of [
    "feature",
    "serial-port",
    "mode-assignment",
    "receiver failsafe",
    "VTX table",
    "VTX activation",
    "adjustment range",
    "CLI collection text",
  ]) {
    const table = page.getByRole("region", {
      name: `Three-backup ${name.toLowerCase()} comparison`,
      exact: true,
    });
    await expect(table).toBeVisible();
    await expect(table.getByRole("columnheader").nth(1)).toContainText(
      "Baseline A:",
    );
  }
  const raw = page.getByRole("region", {
    name: "Three-backup cli collection text comparison",
    exact: true,
  });
  await expect(raw).toContainText("C only text");

  await page.getByLabel("Filter compared parameters").fill("thr_expo");
  await page.getByLabel("Rate profile C", { exact: true }).selectOption("1");
  await expect(comparison.getByRole("row").nth(1)).toContainText("C only");
  const original = await throttle.locator("path").first().getAttribute("d");
  await expect(throttle.locator("path").nth(2)).not.toHaveAttribute(
    "d",
    original!,
  );
  await page.getByLabel("Comparison baseline").selectOption("2");
  await expect(ranges.getByRole("row").nth(1)).toContainText("Same change");
  await expect(ranges.getByRole("columnheader").nth(1)).toContainText(
    "Baseline C:",
  );

  await expect(comparison.getByRole("row").nth(1)).toContainText("Same change");
  await expect(comparison.getByRole("columnheader").nth(1)).toContainText(
    "Baseline C:",
  );
  await expect(throttle.locator("path").first()).toHaveAttribute(
    "d",
    original!,
  );
  await page.getByLabel("Rate profile C", { exact: true }).selectOption("2");
  await expect(throttle.locator("path")).toHaveCount(2);
  await expect(throttle).toContainText("thr_mid=100");
  await page.getByLabel("Backup C", { exact: true }).selectOption("");
  await expect(comparison).toHaveCount(0);
  await expect(
    page.getByRole("region", { name: "Parameter comparison", exact: true }),
  ).toBeVisible();
  await expect(throttle.locator("path")).toHaveCount(2);
});

test("portable session round-trips graph order, baseline and profiles", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    let saved: {
      profiles: {
        documentId: string;
        rateProfile: number;
        pidProfile: number;
      }[];
      comparison: { documents: string[]; baseline: string | null } | null;
      activeId: string | null;
      tab: string;
      theme: string;
    } | null = null;
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (
        command: string,
        args: {
          sourceId?: string;
          configId?: string;
          rateProfile?: number;
          request?: NonNullable<typeof saved>;
        },
      ) => {
        if (command === "restore_session")
          return {
            sources: fixtures.map((f, i) => ({
              id: String(i),
              label: f.artifact.document.title,
            })),
            unavailable: [],
          };
        if (command === "pending_sources") return [];
        if (command === "open_source")
          return fixtures[Number(args.sourceId)].artifact;
        if (command === "inspect_config")
          return fixtures.find((f) => f.artifact.document.id === args.configId)
            .inspections[args.rateProfile!];
        if (command === "save_portable_session") {
          saved = args.request!;
          return true;
        }
        if (command === "open_portable_session") {
          if (!saved) throw "No saved session";
          return {
            ...saved,
            artifacts: fixtures.map((f) => f.artifact),
            workspace: null,
            warnings: [],
          };
        }
        return null;
      },
    };
  }, fixtures);
  await page.goto("/");
  await expect(page.locator(".document")).toHaveCount(3);
  await page
    .getByRole("button", { name: "Compare backups", exact: true })
    .click();
  // Clear A before assigning it to C, then put the third backup in A.
  await page.getByLabel("Backup A", { exact: true }).selectOption("");
  await page
    .getByLabel("Backup C", { exact: true })
    .selectOption(fixtures[0].artifact.document.id);
  await page
    .getByLabel("Backup A", { exact: true })
    .selectOption(fixtures[2].artifact.document.id);
  await page.getByLabel("Comparison baseline").selectOption("1");
  await page.getByLabel("Rate profile A", { exact: true }).selectOption("1");
  const pid = String(fixtures[1].artifact.document.pidProfiles.at(-1));
  await page.getByLabel("PID profile B", { exact: true }).selectOption(pid);
  await page.getByText("Portable sessions", { exact: true }).click();
  await page.getByRole("button", { name: "Save session as…" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Saved session references" }),
  ).toBeVisible();
  await page.getByLabel("Rate profile A", { exact: true }).selectOption("0");
  await page.getByLabel("Comparison baseline").selectOption("0");
  await page.getByLabel("Backup C", { exact: true }).selectOption("");
  await page
    .getByRole("button", { name: "Back to inspector", exact: true })
    .click();
  await page.getByRole("button", { name: "Open session…" }).click();
  await expect(page.getByLabel("Backup A", { exact: true })).toHaveValue(
    fixtures[2].artifact.document.id,
  );
  await expect(page.getByLabel("Backup B", { exact: true })).toHaveValue(
    fixtures[1].artifact.document.id,
  );
  await expect(page.getByLabel("Backup C", { exact: true })).toHaveValue(
    fixtures[0].artifact.document.id,
  );
  await expect(page.getByLabel("Comparison baseline")).toHaveValue("1");
  await expect(page.getByLabel("Rate profile A", { exact: true })).toHaveValue(
    "1",
  );
  await expect(page.getByLabel("PID profile B", { exact: true })).toHaveValue(
    pid,
  );
  await expect(
    page
      .getByRole("region", { name: "roll comparison", exact: true })
      .locator("path"),
  ).toHaveCount(3);
  await page
    .getByRole("button", { name: "Back to inspector", exact: true })
    .click();
  // The last imported backup is A; its inspector uses the same saved rate profile.
  await page.locator(".document").nth(2).getByRole("button").first().click();
  await page.getByRole("tab", { name: "Rates", exact: true }).click();
  await expect(
    page.getByText("Detailed view: Profile 2.", { exact: false }),
  ).toBeVisible();
});
