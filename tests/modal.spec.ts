import { test, expect } from "@playwright/test";

for (const kind of ["paste", "feedback"] as const) {
  test(`${kind} dialog contains keyboard focus and restores its opener`, async ({
    page,
  }) => {
    await page.addInitScript(() => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (command: string) => {
          if (command === "restore_session")
            return { sources: [], unavailable: [] };
          if (command === "pending_sources") return [];
          if (command === "feedback_report") throw "Enter a subject.";
          return 1;
        },
      };
    });
    await page.goto("/");
    const opener = page.getByRole("button", {
      name: kind === "paste" ? /Paste configuration/ : "Send feedback",
    });
    await opener.click();
    const dialog = page.getByRole("dialog");
    await expect(
      dialog.getByLabel(kind === "paste" ? "Backup text" : "Subject", {
        exact: true,
      }),
    ).toBeFocused();
    const close = dialog.getByRole("button", { name: `Close ${kind} dialog` });
    const cancel = dialog.getByRole("button", { name: "Cancel", exact: true });
    await close.focus();
    await page.keyboard.press("Shift+Tab");
    await expect(cancel).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(close).toBeFocused();
    // The global shortcut must not stack a second modal.
    await page.keyboard.press("Control+Shift+V");
    await expect(dialog).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(opener).toBeFocused();
    await opener.click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Cancel", exact: true })
      .click();
    await expect(opener).toBeFocused();
  });
}
