import { test, expect } from "@playwright/test";

test("portable session controls preserve state on cancel and report partial restore", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    let opens = 0;
    let saves = 0;
    const workspace = {
      root: "/moved/backups",
      status: "complete",
      visited: 1,
      skipped: 0,
      indexed: 1,
      total: 1,
      offset: 0,
      entries: [
        {
          id: "restored:0",
          relativePath: "quad.dump",
          bytes: 1024,
          modifiedSeconds: null,
        },
      ],
    };
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (
        command: string,
        args: {
          retry?: boolean;
          relink?: boolean;
          request?: {
            preserveUnavailableSelections: boolean;
            documentIds: string[];
            activeId: string;
            tab: string;
            theme: string;
          };
        },
      ) => {
        if (command === "restore_session")
          return { sources: [], unavailable: [] };
        if (command === "workspace_page") return workspace;
        if (command === "pending_sources") return [];
        if (command === "open_portable_session") {
          if (args.retry) win.retriedSession = true;
          if (args.relink && args.retry) win.relinkedSession = true;
          if (++opens === 2) return null;
          if (opens === 3)
            throw "Unsupported FlightLens session format or version";
          return {
            workspace: {
              ...workspace,
              status: "scanning",
              entries: [],
              indexed: 0,
              total: 0,
            },
            artifacts: [
              {
                kind: "recognized",
                document: {
                  id: "backup-a",
                  title: "Session backup",
                  family: "inav",
                  message: "Recognized backup",
                },
              },
            ],
            activeId: "backup-a",
            tab: "Raw",
            theme: "light",
            warnings: ["missing.dump: Cannot open selected source"],
          };
        }
        if (command === "save_portable_session") {
          const request = args.request!;
          if (
            request.preserveUnavailableSelections !== (++saves === 1) ||
            request.documentIds.join() !== "backup-a" ||
            request.activeId !== "backup-a" ||
            request.tab !== "Raw" ||
            request.theme !== "light"
          )
            throw "Incorrect session state";
          return true;
        }
        return null;
      },
    };
  });
  await page.goto("/");
  await page.getByText("Session options", { exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Retry session", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Relink session backups…" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Open session…" }).click();
  await expect(
    page.getByRole("navigation", { name: "Open documents" }),
  ).toContainText("Session backup");
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: /missing.dump|Saved session references/ }),
  ).toContainText("missing.dump");
  const explorer = page.getByRole("region", { name: "Workspace explorer" });
  await expect(explorer).toContainText("/moved/backups");
  await expect(
    explorer.getByRole("button", { name: /quad.dump/ }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Save session as…" }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: /missing.dump|Saved session references/ }),
  ).toContainText("Saved session references");
  const keep = page.getByRole("checkbox", {
    name: "Keep unavailable active/comparison selections when saving",
  });
  await expect(keep).toBeChecked();
  await keep.uncheck();
  await page.getByRole("button", { name: "Save session as…" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Saved session references" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Retry session", exact: true })
    .click();
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as unknown as Record<string, unknown>).retriedSession,
      ),
    )
    .toBe(true);
  await expect(keep).not.toBeChecked();
  await expect(
    page.getByRole("navigation", { name: "Open documents" }),
  ).toContainText("Session backup");
  await page.getByRole("button", { name: "Open session…" }).click();
  await expect(
    page.getByRole("button", { name: "Retry session", exact: true }),
  ).toBeEnabled();
  await expect(explorer).toContainText("/moved/backups");
  await expect(page.getByRole("alert")).toContainText(
    "Unsupported FlightLens session",
  );
  await expect(
    page.getByRole("navigation", { name: "Open documents" }),
  ).toContainText("Session backup");
  await page
    .getByRole("button", { name: "Relink session backups…", exact: true })
    .click();
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as unknown as Record<string, unknown>).relinkedSession,
      ),
    )
    .toBe(true);
  await expect(keep).toBeChecked();
  await expect(
    page.getByRole("status").filter({ hasText: "Opened 1 session backups." }),
  ).toBeVisible();
});
