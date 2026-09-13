import { test, expect } from "@playwright/test";

test("workspace discovery, search, pages, cancellation, reconnect and opening", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const win = window as unknown as Record<string, unknown>;
    win.isTauri = true;
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    let status = "scanning";
    let disconnected = false;
    let generation = 0;
    const entries = Array.from({ length: 75 }, (_, i) => ({
      id: String(i),
      relativePath: `nested/backup-${String(i).padStart(2, "0")}.dump`,
      bytes: 1024,
      modifiedSeconds: null,
    }));
    const result = (query = "", offset = 0) => {
      const matching = entries.filter((e) => e.relativePath.includes(query));
      return {
        root: "/removable/backups",
        status,
        visited: 76,
        skipped: 0,
        indexed: 75,
        total: matching.length,
        offset,
        entries: matching.slice(offset, offset + 50),
      };
    };
    win.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (
        command: string,
        args: {
          query?: string;
          offset?: number;
          refresh?: boolean;
          entryId?: string;
        },
      ) => {
        if (command === "restore_session")
          return { sources: [], unavailable: [] };
        if (command === "pending_sources") return [];
        if (command === "choose_workspace") {
          if (args.refresh && disconnected) {
            disconnected = false;
            throw "Workspace folder is unavailable. Reconnect the drive and retry.";
          }
          generation++;
          status = generation === 1 ? "scanning" : "complete";
          return result();
        }
        if (command === "workspace_page") {
          // A late response for an obsolete query must not overwrite the new search.
          if (args.query === "slow")
            await new Promise((resolve) => setTimeout(resolve, 500));
          return result(args.query, args.offset);
        }
        if (command === "cancel_workspace") {
          status = "cancelled";
          return;
        }
        if (command === "open_workspace_entry") {
          if (args.entryId === "50") {
            disconnected = true;
            throw "Backup unavailable. Reconnect the drive or refresh the workspace.";
          }
          return {
            kind: "recognized",
            document: {
              id: "opened",
              title: "backup-00.dump",
              family: "inav",
              message: "Unsupported configuration",
            },
          };
        }
        return 1;
      },
    };
  });
  await page.goto("/");
  const explorer = page.getByRole("region", { name: "Workspace explorer" });
  await explorer
    .getByRole("button", { name: "Choose workspace folder…" })
    .click();
  await expect(
    explorer.getByRole("navigation").getByRole("button"),
  ).toHaveCount(50);
  await expect(
    page
      .getByRole("navigation", { name: "Open documents" })
      .getByRole("button"),
  ).toHaveCount(0);
  await page.screenshot({
    path: test.info().outputPath("workspace-explorer.png"),
  });
  await explorer.getByRole("button", { name: "Cancel scan" }).click();
  await expect(explorer.getByRole("status")).toContainText("cancelled");
  await explorer.getByRole("button", { name: "Next files" }).click();
  await expect(
    explorer.getByRole("navigation").getByRole("button"),
  ).toHaveCount(25);
  await explorer.getByRole("button", { name: /nested\/backup-50/ }).click();
  await expect(explorer.getByRole("alert")).toContainText("Backup unavailable");
  await explorer.getByRole("button", { name: "Refresh workspace" }).click();
  await expect(explorer.getByRole("alert")).toContainText(
    "Workspace folder is unavailable",
  );
  await explorer.getByRole("button", { name: "Refresh workspace" }).click();
  await expect(explorer.getByRole("status")).toContainText("complete");
  await explorer.getByRole("textbox").fill("slow");
  await page.waitForTimeout(300);
  await explorer.getByRole("textbox").fill("backup-00");
  await expect(
    explorer.getByRole("navigation").getByRole("button"),
  ).toHaveCount(1);
  await explorer.getByRole("button", { name: /nested\/backup-00/ }).click();
  await expect(
    page.getByRole("navigation", { name: "Open documents" }),
  ).toContainText("backup-00.dump");
  await explorer.getByRole("textbox").fill("missing");
  await expect(
    explorer.getByRole("navigation").getByRole("button"),
  ).toHaveCount(0);
});
