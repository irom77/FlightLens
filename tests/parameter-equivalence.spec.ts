import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixtures = [[], ["--cross-version-parameters"]].map((flags) =>
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
        "--parameter-baseline",
        ...flags,
      ],
      { encoding: "utf8" },
    ),
  ),
);

test("certified parameter comparisons use real parser firmware identities", async ({
  page,
}) => {
  expect(fixtures.map((f) => f.artifact.document.firmware.family)).toEqual([
    "betaflight",
    "betaflight",
  ]);
  expect(fixtures.map((f) => f.artifact.document.firmware.version)).toEqual([
    "4.5.0",
    "2025.12.1",
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
    name: "Parameter comparison",
    exact: true,
  });
  await parameters.getByLabel("Hide equal values").uncheck();
  for (const [key, status] of [
    ["motor_poles", "Changed"],
    ["vbat_max_cell_voltage", "Equal"],
    ["vbat_min_cell_voltage", "Equal"],
    ["vbat_warning_cell_voltage", "Equal"],
    ["motor_pwm_protocol", "Not comparable"],
  ]) {
    await parameters.getByLabel("Filter compared parameters").fill(key);
    await expect(parameters.getByRole("row").nth(1)).toContainText(status);
  }
});
