import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixtures = [[], ["--zero-expo"], ["--vendor-missing-expo"]].map((flags) =>
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
        ...flags,
      ],
      { encoding: "utf8" },
    ),
  ),
);

test("compares backend curves, independent profiles, unknown inputs and closed documents", async ({
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
  const roll = page.getByRole("region", {
    name: "roll comparison",
    exact: true,
  });
  await expect(roll.locator("path")).toHaveCount(2);
  const paths = await roll
    .locator("path")
    .evaluateAll((nodes) => nodes.map((n) => n.getAttribute("d")));
  expect(paths[0]).not.toEqual(paths[1]);
  await page.screenshot({
    path: "test-results/comparison.png",
    fullPage: true,
  });
  await expect(roll.locator("path").nth(1)).toHaveAttribute(
    "stroke-dasharray",
    "7 3",
  );
  await roll.locator("svg").hover({ position: { x: 300, y: 100 } });
  await expect(roll.locator(".legend")).toContainText("°/s");
  const profiles = Object.keys(fixtures[0].inspections);
  expect(profiles.length).toBeGreaterThan(1);
  await page
    .getByLabel("Rate profile A", { exact: true })
    .selectOption(profiles[1]);
  await expect(page.getByLabel("Rate profile B", { exact: true })).toHaveValue(
    "0",
  );
  await expect(roll.locator(".legend")).toContainText(
    `Profile ${Number(profiles[1]) + 1}`,
  );
  await page
    .getByLabel("Backup B", { exact: true })
    .selectOption(fixtures[2].artifact.document.id);
  await expect(roll.locator("path")).toHaveCount(1);
  await expect(roll).toContainText("unknown");
  await page
    .getByRole("button", {
      name: `Close ${fixtures[0].artifact.document.title}`,
      exact: true,
    })
    .first()
    .click();
  await expect(
    page.getByText("Select two available backups.", { exact: false }),
  ).toBeVisible();
  await expect(page.locator("main path")).toHaveCount(0);
  await page
    .getByLabel("Backup A", { exact: true })
    .selectOption(fixtures[1].artifact.document.id);
  await expect(roll.locator("path")).toHaveCount(1);
});
