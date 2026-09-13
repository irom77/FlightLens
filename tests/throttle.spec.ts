import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

for (const vendor of [false, true]) {
  test(`throttle preview uses imported profiles and provenance; vendor=${vendor}`, async ({
    page,
  }) => {
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
          "--throttle-preview",
          ...(vendor ? ["--vendor-missing-expo"] : []),
        ],
        { encoding: "utf8" },
      ),
    );
    await page.addInitScript((f) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (command: string, args: { rateProfile?: number }) => {
          if (command === "pending_sources") return [];
          if (command === "restore_session")
            return { sources: [], unavailable: [] };
          if (command === "ingest_text") return f.artifact;
          if (command === "inspect_config")
            return f.inspections[args.rateProfile!];
          return 1;
        },
      };
    }, fixture);
    await page.goto("/");
    await page
      .getByRole("button", { name: "Explore a synthetic example" })
      .click();
    const preview = page.getByRole("region", {
      name: "Throttle Curve Preview",
    });
    await expect(preview).toContainText("SCALE");
    await expect(preview.getByRole("textbox")).toHaveCount(0);
    if (vendor) {
      await expect(preview).toContainText(
        "not verified for this firmware release",
      );
      await expect(preview.locator("path")).toHaveCount(0);
      return;
    }
    await expect(preview.getByRole("img")).toHaveAccessibleName(
      /normalized throttle input.*throttle command/,
    );
    await expect(preview.locator("path")).toHaveCount(1);
    const first = await preview.locator("path").getAttribute("d");
    await page.getByRole("button", { name: /^Profile 2 / }).click();
    await expect(preview).toContainText("CLIP");
    await expect(preview.locator("path")).not.toHaveAttribute("d", first!);
    const origin = preview.getByRole("button", { name: /^Line / }).first();
    const line = (await origin.innerText()).split(" ")[1];
    await origin.click();
    await expect(page.locator(`#line-${line}`)).toHaveClass("highlight-line");
    await page.getByRole("tab", { name: "Rates", exact: true }).click();
    await page.getByRole("button", { name: /^Profile 3 / }).click();
    await expect(preview).toContainText("thr_mid=100");
    await expect(preview.locator("path")).toHaveCount(0);
    await page.getByRole("button", { name: /^Profile 4 / }).click();
    await expect(preview).toContainText("thr_expo is missing");
    await expect(preview.locator("path")).toHaveCount(0);
  });
}
