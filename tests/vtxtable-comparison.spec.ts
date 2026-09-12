import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixtures = [["--vtxtable-baseline"], ["--vtxtable-differences"]].map(
  (flags) =>
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
          "--explicit-only",
          ...flags,
        ],
        { encoding: "utf8" },
      ),
    ),
);

test("VTX table comparison tracks parser declarations and invalidation", async ({
  page,
}) => {
  expect(fixtures.map((f) => f.artifact.document.firmware.family)).toEqual([
    "betaflight",
    "betaflight",
  ]);
  expect(fixtures.map((f) => f.artifact.document.firmware.version)).toEqual([
    "4.5.0",
    "4.5.0",
  ]);
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
        if (command === "inspect_config")
          return fixtures.find((f) => f.artifact.document.id === args.configId)
            .inspections[args.rateProfile!];
        return 1;
      },
    };
  }, fixtures);
  await page.goto("/");
  await expect(page.locator(".document")).toHaveCount(2);
  await page
    .getByRole("button", { name: "Compare backups", exact: true })
    .click();
  const parameters = page.getByRole("region", {
    name: "VTX table comparison",
    exact: true,
  });
  await parameters.getByLabel("Hide equal VTX table entries").uncheck();
  for (const [key, status] of [
    ["band 1", "Equal"],
    ["powervalues", "Changed"],
    ["powerlabels", "Unknown"],
  ]) {
    await parameters.getByLabel("Filter compared VTX table entries").fill(key);
    const row = parameters
      .getByRole("row")
      .filter({ has: page.getByRole("cell", { name: key, exact: true }) });
    await expect(row).toContainText(status);
    if (key === "band 1") {
      await expect(row).toContainText("RACE R FACTORY 5658 5695");
      await row.getByText(/B: Line/).click();
      await expect(row.locator("pre").last()).toHaveText(
        "vtxtable band 1 race r factory 05658 5695",
      );
    }
  }
});
