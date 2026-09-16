import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixtures = [
  [],
  ["--zero-expo"],
  ["--vendor-missing-expo", "--unverified-throttle"],
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
        ...flags,
      ],
      { encoding: "utf8" },
    ),
  ),
);

test("overlays throttle profiles and preserves supported curves beside unavailable backups", async ({
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
  const throttle = page.getByRole("region", {
    name: "Throttle comparison",
    exact: true,
  });
  await expect(throttle.locator("path")).toHaveCount(2);
  await expect(throttle.locator(".legend")).toContainText("A:");
  await expect(throttle.locator(".legend")).toContainText("B:");
  const first = await throttle.locator("path").first().getAttribute("d");
  await page.getByLabel("Rate profile B", { exact: true }).selectOption("1");
  await expect(throttle.locator("path").first()).toHaveAttribute("d", first!);
  await expect(throttle.locator("path").nth(1)).not.toHaveAttribute(
    "d",
    first!,
  );
  await page
    .getByLabel("Backup B", { exact: true })
    .selectOption(fixtures[2].artifact.document.id);
  await expect(throttle.locator("path")).toHaveCount(1);
  await expect(throttle).toContainText(
    "not verified for this firmware release",
  );
  await expect(
    page
      .getByRole("region", { name: "roll comparison", exact: true })
      .locator("path"),
  ).toHaveCount(1);
});
