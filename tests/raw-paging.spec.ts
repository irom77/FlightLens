import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixture = JSON.parse(
  execFileSync(
    process.env.FLIGHTLENS_CARGO ?? "cargo",
    ["run", "--quiet", "-p", "flightlens-core", "--bin", "preview_fixture"],
    { encoding: "utf8" },
  ),
);

test("compact sessions page source, ignore stale replies, retry and reload", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.addInitScript((f) => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    const artifact = f.artifact;
    if ("syntax" in artifact.document)
      throw new Error("Import must be compact");
    artifact.document.sourceId = "file-source";
    artifact.document.sourceEvidence.lineCount = 1002;
    let generation = 0;
    const pending: {
      offset: number;
      resolve: (value: unknown) => void;
      reject: (reason: string) => void;
      generation: number;
    }[] = [];
    win.finishRaw = (offset: number, error = false) => {
      const index = pending.findIndex((request) => request.offset === offset);
      if (index < 0) throw new Error(`No request for ${offset}`);
      const request = pending.splice(index, 1)[0];
      if (error) request.reject("Source temporarily unavailable");
      else
        request.resolve(
          Array.from({ length: Math.min(500, 1002 - offset) }, (_, i) => ({
            line: offset + i + 1,
            raw: `source generation ${request.generation} line ${offset + i + 1}\r\n`,
            command: { kind: "comment" },
            offset: 0,
          })),
        );
    };
    win.rawOffsets = () => pending.map((request) => request.offset);
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (
        command: string,
        args: { offset: number; count: number },
      ) => {
        if (command === "restore_session")
          return { sources: [], unavailable: [] };
        if (command === "pending_sources") return [];
        if (command === "open_portable_session")
          return {
            artifacts: [artifact],
            profiles: [],
            comparison: null,
            activeId: artifact.document.id,
            tab: "Raw",
            theme: "dark",
            warnings: [],
            workspace: null,
          };
        if (command === "open_source") {
          generation++;
          return structuredClone(artifact);
        }
        if (command === "inspect_config") return f.inspection;
        if (command === "raw_page") {
          if (args.count !== 500) throw new Error("Unbounded raw request");
          return new Promise((resolve, reject) =>
            pending.push({ offset: args.offset, resolve, reject, generation }),
          );
        }
        return 1;
      },
    };
  }, fixture);
  const finish = async (offset: number, error = false) => {
    await expect
      .poll(() =>
        page.evaluate(() =>
          (window as unknown as { rawOffsets: () => number[] }).rawOffsets(),
        ),
      )
      .toContain(offset);
    await page.evaluate(
      ({ offset, error }) =>
        (
          window as unknown as {
            finishRaw: (offset: number, error: boolean) => void;
          }
        ).finishRaw(offset, error),
      { offset, error },
    );
  };
  await page.goto("/");
  await page
    .getByRole("button", { name: "Open session…", exact: true })
    .click();
  await expect(page.getByText("Loading source lines…")).toBeVisible();
  await page.getByRole("button", { name: "Next lines" }).click();
  await finish(500);
  await expect(page.locator("#line-501")).toContainText("generation 0");
  await finish(0);
  await expect(page.locator("#line-1")).toHaveCount(0);
  await expect(page.locator("#line-501")).toBeVisible();
  await page.getByRole("button", { name: "Next lines" }).click();
  await finish(1000, true);
  await expect(
    page.getByRole("alert", { name: "Source loading error" }),
  ).toContainText("Source temporarily unavailable");
  await page.getByRole("button", { name: "Retry source lines" }).click();
  await finish(1000);
  await expect(page.locator(".raw-source > div")).toHaveCount(2);
  await expect(page.getByRole("button", { name: "Next lines" })).toBeDisabled();
  await page.getByRole("button", { name: "Previous lines" }).click();
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as unknown as { rawOffsets: () => number[] }).rawOffsets(),
      ),
    )
    .toContain(500);
  await page.getByRole("button", { name: "Reload file" }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as unknown as { rawOffsets: () => number[] })
            .rawOffsets()
            .filter((n) => n === 500).length,
      ),
    )
    .toBe(2);
  await finish(500); // old snapshot reply cannot repaint the reloaded document
  await expect(page.locator(".raw-source > div")).toHaveCount(0);
  await finish(500);
  await expect(page.locator("#line-501")).toContainText("generation 1");
  await page.getByRole("button", { name: "Previous lines" }).click();
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as unknown as { rawOffsets: () => number[] }).rawOffsets(),
      ),
    )
    .toContain(0);
  await page
    .getByRole("button", { name: /^Close Synthetic smoke fixture$/ })
    .click();
  await finish(0);
  await expect(page.locator(".raw-source")).toHaveCount(0);
  expect(errors).toEqual([]);
});
