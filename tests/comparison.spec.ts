import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixtures = [
  ["--explicit-only"],
  [
    "--explicit-only",
    "--zero-expo",
    "--feature-differences",
    "--port-differences",
    "--mode-differences",
    "--collection-differences",
  ],
  ["--vendor-missing-expo"],
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
        "--collection-baseline",
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
  const receiverRanges = page.getByRole("region", {
    name: "Receiver-range comparison",
    exact: true,
  });
  await expect(receiverRanges.getByRole("row").nth(1)).toContainText("Changed");
  await expect(receiverRanges.getByRole("row").nth(1)).toContainText("1050 µs");
  await receiverRanges.getByText(/^B: Line/).click();
  await expect(receiverRanges.locator("pre").last()).toHaveText(
    "rxrange 0 1050 1950",
  );
  await receiverRanges
    .getByLabel("Filter compared receiver channels")
    .fill("3");
  await expect(receiverRanges.getByRole("row")).toHaveCount(1);
  await receiverRanges.getByLabel("Filter compared receiver channels").fill("");
  const collections = page.getByRole("region", {
    name: "CLI collection text comparison",
    exact: true,
  });
  await expect(collections).toBeVisible();
  await collections.getByLabel("Hide matching text").uncheck();
  await collections.getByLabel("Filter compared collections").fill("vtxtable");
  await expect(collections.getByRole("row").nth(1)).toContainText(
    "Matching text",
  );
  await collections.getByLabel("Hide matching text").check();
  await expect(collections.getByRole("row")).toHaveCount(1);
  await collections.getByLabel("Filter compared collections").fill("rxrange");
  const rxrange = collections.getByRole("row").nth(1);
  await expect(rxrange).toContainText("Different text");
  await rxrange
    .getByText(/^B: Line/)
    .last()
    .click();
  await expect(rxrange.locator("pre").last()).toHaveText("rxrange 0 1050 1950");
  await collections.getByLabel("Filter compared collections").fill("adjrange");
  await expect(collections.getByRole("row").nth(1)).toContainText("Unknown");
  const parameters = page.getByRole("region", {
    name: "Parameter comparison",
    exact: true,
  });
  await expect(parameters).toBeVisible();
  const features = page.getByRole("region", {
    name: "Feature comparison",
    exact: true,
  });
  await expect(features).toBeVisible();
  const receiver = features.getByRole("row").filter({ hasText: "RX_SERIAL" });
  await expect(receiver).toHaveCount(0);
  await features.getByLabel("Hide equal features").uncheck();
  await expect(receiver).toContainText("Equal");
  await features.getByLabel("Hide equal features").check();
  const osd = features.getByRole("row").filter({ hasText: "OSD" });
  await expect(osd).toContainText("Changed");
  await expect(osd).toContainText("Disabled");
  await osd.getByText(/^B: Line/).click();
  await expect(osd.locator("pre").nth(1)).toHaveText("feature -OSD");
  await expect(
    features.getByRole("row").filter({ hasText: "3D" }),
  ).toContainText("Unknown");
  await features.getByLabel("Filter compared features").fill("osd");
  await expect(features.getByRole("row")).toHaveCount(2);
  await features.getByLabel("Filter compared features").fill("");
  const ports = page.getByRole("region", {
    name: "Serial-port comparison",
    exact: true,
  });
  await expect(ports).toBeVisible();
  await ports.getByLabel("Hide equal ports").uncheck();
  await expect(
    ports.getByRole("row").filter({ hasText: "USB VCP" }),
  ).toContainText("Equal");
  await ports.getByLabel("Hide equal ports").check();
  await ports.getByLabel("Filter compared ports").fill("UART 1");
  const uart = ports.getByRole("row").filter({ hasText: "UART 1" });
  await expect(uart).toContainText("Changed");
  await expect(uart).toContainText("230400");
  await uart.getByText(/^B: Line/).click();
  await expect(uart.locator("pre").nth(1)).toContainText("serial 0 64 230400");
  await ports.getByLabel("Filter compared ports").fill("UART 2");
  await expect(ports.getByRole("row").nth(1)).toContainText("Unknown");
  await ports.getByLabel("Filter compared ports").fill("UART 1");
  const modes = page.getByRole("region", {
    name: "Mode-assignment comparison",
    exact: true,
  });
  await modes.getByLabel("Hide equal modes").uncheck();
  await expect(
    modes.getByRole("row").filter({ hasText: "ANGLE" }),
  ).toContainText("Equal");
  await modes.getByLabel("Hide equal modes").check();
  await modes.getByLabel("Filter compared modes").fill("ARM");
  const arm = modes.getByRole("row").filter({ hasText: "ARM" });
  await expect(arm).toContainText("Changed");
  await expect(arm).toContainText("AUX 2");
  await arm.getByText(/^B: Line/).click();
  await expect(arm.locator("pre").nth(1)).toHaveText("aux 0 0 1 1600 2100 1 0");
  await modes.getByLabel("Filter compared modes").fill("2");
  await expect(modes.getByRole("row").nth(1)).toContainText("Unknown");
  await expect(modes.getByRole("row").nth(1)).toContainText(
    "Unassigned channel (14)",
  );
  await modes.getByLabel("Filter compared modes").fill("ARM");
  await parameters.getByLabel("Filter compared parameters").fill("roll_expo");
  const expo = parameters.getByRole("row").filter({ hasText: "roll_expo" });
  await expect(expo).toContainText("Changed");
  await expo.getByText(/^A: Line/).click();
  await expect(expo.locator("pre").first()).toContainText("set roll_expo");
  await page.getByLabel("PID profile A", { exact: true }).selectOption("1");
  await expect(page.getByLabel("PID profile B", { exact: true })).toHaveValue(
    "0",
  );
  await expect(page.getByLabel("Rate profile A", { exact: true })).toHaveValue(
    "0",
  );
  await parameters.getByLabel("Filter compared parameters").fill("p_roll");
  await expect(
    parameters.getByRole("row").filter({ hasText: "p_roll" }),
  ).toContainText("Unknown");
  await parameters.getByLabel("Filter compared parameters").fill("");
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
  await expect(osd).toContainText("Not comparable");
  await expect(uart).toContainText("Not comparable");
  await expect(arm).toContainText("Not comparable");
  await expect(receiverRanges.getByRole("row").nth(1)).toContainText(
    "Not comparable",
  );
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
