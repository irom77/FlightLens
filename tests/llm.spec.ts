import { expect, test, type Page } from "@playwright/test";
import { aiFixture } from "../tools/llmFixture";
const calls = (page: Page) =>
  page.evaluate(() => (window as unknown as { __aiCalls: string[] }).__aiCalls);
test("summary is explicit, reviewed, plain text and exportable", async ({
  page,
}) => {
  await aiFixture(page);
  expect(await calls(page)).not.toContain("llm_summarize");
  await page.getByRole("button", { name: "AI summary", exact: true }).click();
  const preview = page.getByRole("dialog", { name: "Review AI request" });
  await expect(preview).toContainText(
    "http://localhost:11434/v1/chat/completions",
  );
  await expect(preview).toContainText("Preserve unknown values");
  await expect(preview).toContainText("craft_name");
  await expect(preview).toContainText("Synthetic backup → Backup A");
  await preview.getByText("Structured digest JSON").click();
  await expect(preview).toContainText('"motor_poles":null');
  expect(await calls(page)).not.toContain("llm_summarize");
  await preview.getByRole("button", { name: "Send to" }).click();
  await expect(preview).toHaveCount(0);
  await expect(page.locator(".ai-result-text")).toContainText(
    "<script>Model output is plain text.</script>",
  );
  await expect(page.locator(".ai-result script")).toHaveCount(0);
  await expect(page.locator(".ai-result")).toContainText(
    "not a certified finding",
  );
  await page.getByRole("button", { name: "Copy", exact: true }).click();
  expect(
    await page.evaluate(
      () => (window as unknown as { __copied: string }).__copied,
    ),
  ).toContain("not a certified finding");
  await page.getByRole("button", { name: "Save as Markdown" }).click();
  await expect(
    page.getByText("Summary saved to a new Markdown file."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Regenerate" }).click();
  await expect(preview).toBeVisible();
  expect((await calls(page)).filter((c) => c === "llm_summarize")).toHaveLength(
    1,
  );
  await preview.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(await calls(page)).toContain("llm_cancel");
  await page.getByRole("tab", { name: "PID", exact: true }).click();
  await expect(page.locator(".ai-result")).toBeVisible();
  await page.getByRole("tab", { name: "Rates", exact: true }).click();
  await page.locator(".profile-card").nth(1).click();
  await expect(page.locator(".ai-result")).toHaveCount(0);
  expect((await calls(page)).filter((c) => c === "llm_summarize")).toHaveLength(
    1,
  );
});
test("settings gate summaries, validate, clear keys and review connection tests", async ({
  page,
}) => {
  await aiFixture(page, "dark", false);
  await expect(
    page.getByRole("button", { name: "AI summary", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText("AI summaries are off. Enable them in settings."),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "AI summary settings", exact: true })
    .click();
  const settings = page.getByRole("dialog", { name: "AI summary settings" });
  await settings.getByLabel("Model ID", { exact: true }).fill("");
  await expect(
    settings.getByRole("button", { name: "Save settings" }),
  ).toBeDisabled();
  await settings.getByLabel("Model ID", { exact: true }).fill("local-model");
  await settings.getByLabel("Base URL").fill("http://remote.example");
  await expect(
    settings.getByRole("button", { name: "Save settings" }),
  ).toBeDisabled();
  await settings.getByLabel("Base URL").fill("http://localhost:11434");
  await settings.getByLabel("Enable AI summaries").check();
  await settings.getByRole("button", { name: "Save settings" }).click();
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__keyFailure = true;
  });
  await settings.getByLabel("New API key").fill("synthetic-test-key");
  await settings.getByRole("button", { name: "Save key", exact: true }).click();
  await expect(settings.getByRole("alert")).toContainText(
    "unavailable or locked",
  );
  await expect(settings.getByLabel("New API key")).toHaveValue("");
  await expect(settings.getByLabel("Use a session-only key")).not.toBeChecked();
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__keyFailure = false;
  });
  await settings.getByLabel("Use a session-only key").check();
  await settings.getByLabel("New API key").fill("synthetic-test-key");
  await settings.getByRole("button", { name: "Save key", exact: true }).click();
  await expect(settings).toContainText("Session-only key (discarded on exit)");
  await settings.getByRole("button", { name: "Clear key" }).click();
  await expect(settings).toContainText("No key configured.");
  await settings
    .getByRole("button", { name: "Review connection test" })
    .click();
  const preview = page.getByRole("dialog", { name: "Review AI request" });
  await expect(preview).toContainText("No backup data is included.");
  expect(await calls(page)).not.toContain("llm_test_connection");
  await preview.getByRole("button", { name: "Send to" }).click();
  await expect(settings).toContainText("Connection succeeded");
});
test("cancelled late response is discarded and failures require new review", async ({
  page,
}) => {
  await aiFixture(page);
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__holdSummary = true;
  });
  await page.getByRole("button", { name: "AI summary", exact: true }).click();
  const preview = page.getByRole("dialog", { name: "Review AI request" });
  await preview.getByRole("button", { name: "Send to" }).click();
  await expect(preview).toContainText("Waiting for the model");
  await preview.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.evaluate(() => {
    const win = window as unknown as {
      __resolveSummary: () => void;
      __holdSummary: boolean;
      __summaryError: boolean;
    };
    win.__resolveSummary();
    win.__holdSummary = false;
    win.__summaryError = true;
  });
  await expect(page.locator(".ai-result")).toHaveCount(0);
  await page.getByRole("button", { name: "AI summary", exact: true }).click();
  await preview.getByRole("button", { name: "Send to" }).click();
  await expect(preview.getByRole("alert")).toContainText("timed out");
  await expect(preview.getByRole("button", { name: "Send to" })).toBeDisabled();
});
test("browser settings explain desktop requirement", async ({ page }) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "AI summary settings", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "AI settings require the desktop app.",
  );
  await expect(page.getByLabel("Enable AI summaries")).toBeDisabled();
});

test("preview problems disable Send and require a new review", async ({
  page,
}) => {
  await aiFixture(page);
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__previewProblems = [
      "Add an API key in settings.",
    ];
  });
  await page.getByRole("button", { name: "AI summary", exact: true }).click();
  const preview = page.getByRole("dialog", { name: "Review AI request" });
  await expect(preview).toContainText("Add an API key in settings.");
  await expect(preview.getByRole("button", { name: "Send to" })).toBeDisabled();
  expect(await calls(page)).not.toContain("llm_summarize");
  await preview.getByRole("button", { name: "Cancel", exact: true }).click();
});

test("Compare reviews selected profiles, exports, and invalidates changed selections", async ({
  page,
}) => {
  await aiFixture(page, "dark", true, true);
  const action = page.getByRole("button", { name: "AI summary", exact: true });
  const preview = page.getByRole("dialog", { name: "Review AI request" });
  await expect(page.locator(".document")).toHaveCount(3);
  await expect(action).toBeEnabled();
  expect(await calls(page)).not.toContain("llm_summarize");
  await page.getByLabel("Rate profile B", { exact: true }).selectOption("1");
  await action.click();
  await expect(preview).toContainText("Backup B");
  expect(await calls(page)).not.toContain("llm_summarize");
  await preview.getByRole("button", { name: "Send to" }).click();
  await expect(page.locator(".ai-result")).toBeVisible();
  await page.getByRole("button", { name: "Save as Markdown" }).click();
  const requests = await page.evaluate(
    () =>
      (
        window as unknown as {
          __aiRequests: {
            command: string;
            request: import("../src/bindings/core").SummaryRequest;
          }[];
        }
      ).__aiRequests,
  );
  const sent = requests.find((r) => r.command === "llm_summarize")!.request;
  expect(sent.kind).toBe("diff");
  if (sent.kind !== "diff") throw new Error("expected comparison");
  expect(sent.slots).toHaveLength(2);
  expect(sent.slots[1].rateProfile).toBe(1);
  expect(sent.baseline).toBeNull();
  expect(sent.rows.length).toBeGreaterThan(0);
  expect(
    requests.find((r) => r.command === "llm_save_summary")!.request,
  ).toEqual(sent);
  await page.getByLabel("PID profile A", { exact: true }).selectOption("1");
  await expect(page.locator(".ai-result")).toHaveCount(0);
  await action.click();
  await preview.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.getByLabel("Backup C", { exact: true }).selectOption({ index: 3 });
  await expect(action).toBeDisabled();
  await expect(
    page.getByText("Choose a baseline to summarize three backups."),
  ).toBeVisible();
  await page.getByLabel("Comparison baseline").selectOption("2");
  await action.click();
  await expect(preview).toContainText('"baseline":2');
  await expect(preview).toContainText("Backup C");
  await preview.getByRole("button", { name: "Send to" }).click();
  await expect(page.locator(".ai-result")).toBeVisible();
  await page.getByLabel("Comparison baseline").selectOption("1");
  await expect(page.locator(".ai-result")).toHaveCount(0);
  await page.getByLabel("Backup A", { exact: true }).selectOption("");
  await expect(action).toBeDisabled();
  expect((await calls(page)).filter((c) => c === "llm_summarize")).toHaveLength(
    2,
  );
});
