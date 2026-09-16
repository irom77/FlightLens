import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
const fixture = JSON.parse(
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
      "--pid-defaults",
    ],
    { encoding: "utf8" },
  ),
);

test("PID defaults show provenance, preserve unknown D gains and invalid declarations", async ({
  page,
}) => {
  await page.addInitScript((f) => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (
        command: string,
        args: { rateProfile?: number; offset?: number; count?: number },
      ) => {
        if (command === "pending_sources") return [];
        if (command === "restore_session")
          return { sources: [], unavailable: [] };
        if (command === "ingest_text") return f.artifact;
        if (command === "inspect_config")
          return f.inspections[args.rateProfile ?? 0];
        if (command === "raw_page")
          return f.rawSyntax.slice(
            args.offset,
            (args.offset ?? 0) + (args.count ?? 500),
          );
        return 1;
      },
    };
  }, fixture);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic example" })
    .click();
  await page.getByRole("tab", { name: "PID", exact: true }).click();
  const panel = page.getByRole("tabpanel", { name: "PID", exact: true });
  await expect(panel.getByRole("button", { name: /Profile 1/ })).toContainText(
    "10 from firmware defaults",
  );
  const roll = panel.getByRole("row", { name: /^roll / });
  await expect(roll).toContainText("45*");
  await expect(roll.getByRole("button")).toHaveCount(0);
  await expect(roll.getByRole("cell").nth(2)).toHaveText("Unknown");
  await expect(panel.locator(".parameter-table").first()).toContainText(
    "not declared, not exported",
  );
  await panel.getByRole("button", { name: /Profile 2/ }).click();
  await expect(
    roll.getByRole("button", { name: "0", exact: true }),
  ).toBeVisible();
  await expect(
    roll.getByRole("button", { name: "invalid", exact: true }),
  ).toBeVisible();
  await expect(roll).not.toContainText("80*");
  await expect(panel.getByRole("button", { name: /Profile 2/ })).toContainText(
    "8 from firmware defaults",
  );
  await roll.getByRole("button", { name: "0", exact: true }).click();
  await expect(
    page.getByRole("tabpanel", { name: "Raw", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".raw-source .highlight-line")).toContainText(
    "set p_roll = 0",
  );
});
