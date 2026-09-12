import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";

const fixture = JSON.parse(
  execFileSync(
    process.env.FLIGHTLENS_CARGO ?? "cargo",
    ["run", "--quiet", "-p", "flightlens-core", "--bin", "preview_fixture"],
    { encoding: "utf8" },
  ),
);

const redacted = "# Betaflight 4.5.0\nset craft_name = <redacted>\n";

test("feedback is reviewed in the app and filed by the reporter", async ({
  page,
}) => {
  const external: string[] = [];
  await page.route("**/*", (route) => {
    if (new URL(route.request().url()).hostname !== "127.0.0.1") {
      external.push(route.request().url());
      return route.abort();
    }
    return route.continue();
  });
  await page.addInitScript(
    ([f, redacted]) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__filed = [];
      win.__pendingReports = [];
      // The page has no clipboard permission in a test browser, and the copy
      // is what the report depends on, so record it instead.
      Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: {
          writeText: async (text: string) => {
            win.__copied = text;
          },
        },
      });
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (
          command: string,
          args: {
            configId?: string | null;
            rateProfile?: number;
            request?: {
              subject: string;
              body: string;
              includeConfig: boolean;
            };
          },
        ) => {
          if (command === "pending_sources" || command === "choose_files")
            return [];
          if (command === "restore_session")
            return { sources: [], unavailable: [] };
          if (command === "ingest_text") return (f as any).artifact;
          if (command === "inspect_config")
            return (f as any).inspections[args.rateProfile!];
          if (command === "feedback_report") {
            // Mirrors the validation the core performs, so the dialog is
            // exercised in both the incomplete and the complete state.
            const r = args.request!;
            if (win.__delayReports) {
              await new Promise<void>((resolve) => {
                (win.__pendingReports as (() => void)[]).push(resolve);
              });
            }
            if (!r.subject.trim()) throw "Enter a subject.";
            if (r.body.trim().length < 20)
              throw "Describe the report in at least 20 characters.";
            return {
              url: "https://github.com/irom77/FlightLens/issues/new?labels=bug",
              issueBody: `### Description\n\n${r.body}\n`,
              clipboard: r.includeConfig ? redacted : null,
              removed: r.includeConfig
                ? [{ key: "craft_name", line: 2, category: "identity" }]
                : [],
              oversized: false,
            };
          }
          if (command === "file_feedback_report") {
            (win.__filed as unknown[]).push(args);
            return null;
          }
          return 1;
        },
      };
    },
    [fixture, redacted] as const,
  );
  await page.goto("/");
  await page
    .getByRole("button", { name: "Explore a synthetic example" })
    .click();
  await page.getByRole("button", { name: "Send feedback" }).click();
  const dialog = page.getByRole("dialog", { name: "Send feedback" });
  const submit = dialog.getByRole("button", { name: "Open GitHub issue" });
  // An empty report cannot be filed, and the dialog says why rather than
  // leaving a disabled button unexplained.
  await expect(submit).toBeDisabled();
  await expect(dialog).toContainText("Enter a subject.");
  await dialog.getByLabel("Subject").fill("Rates preview stays blank");
  await expect(dialog).toContainText("at least 20 characters");
  await expect(submit).toBeDisabled();
  await dialog
    .getByLabel("Description")
    .fill("The rates preview stays blank after importing this backup.");
  await expect(submit).toBeEnabled();
  // Nothing is attached until the reporter asks for it, and what would be
  // attached is shown before it leaves the app.
  await expect(dialog).not.toContainText("<redacted>");
  await dialog.getByLabel("Attach the open configuration").check();
  await expect(dialog.locator(".feedback-preview")).toContainText(
    "set craft_name = <redacted>",
  );
  await expect(dialog.locator(".feedback-removed")).toContainText(
    "Line 2: craft_name (name)",
  );
  // Hold IPC responses so edits cannot accidentally submit a previous preview.
  await page.evaluate(() => {
    (window as any).__delayReports = true;
  });
  await dialog.getByLabel("Attach the open configuration").uncheck();
  await expect(submit).toBeDisabled();
  await expect(dialog.locator(".feedback-preview")).toHaveCount(0);
  await dialog.getByLabel("Attach the open configuration").check();
  await dialog
    .getByLabel("Description")
    .fill("Updated description that must appear in the reviewed report.");
  await expect(submit).toBeDisabled();
  await expect
    .poll(() => page.evaluate(() => (window as any).__pendingReports.length))
    .toBe(3);
  // Resolve the latest request first, then stale requests in reverse order.
  await page.evaluate(() => {
    (window as any).__pendingReports.pop()();
  });
  await expect(submit).toBeEnabled();
  await expect(dialog.locator(".feedback-preview")).toContainText(
    "Updated description",
  );
  await page.evaluate(() => {
    for (const resolve of (window as any).__pendingReports.reverse()) resolve();
  });
  await expect(dialog.locator(".feedback-preview")).toContainText(
    "Updated description",
  );
  expect(await page.evaluate(() => (window as any).__copied)).toBeUndefined();
  await submit.click();
  await expect(dialog).toContainText("open in your browser");
  expect(await page.evaluate(() => (window as any).__copied)).toBe(redacted);
  expect(await page.evaluate(() => (window as any).__filed)).toMatchObject([
    {
      configId: fixture.artifact.document.id,
      request: {
        kind: "bug",
        subject: "Rates preview stays blank",
        includeConfig: true,
      },
    },
  ]);
  // Filing happens in the reporter's browser, through the shell. The page
  // itself reaches nothing.
  expect(external).toEqual([]);
});
