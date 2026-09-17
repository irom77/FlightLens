import { execFileSync } from "node:child_process";
import type { Page } from "@playwright/test";
import type {
  ArtifactView,
  Inspection,
  LlmPreview,
  LlmSettings,
  LlmStatus,
  LlmSummary,
  SummaryRequest,
} from "../src/bindings/core";
const fixture: { artifact: ArtifactView; inspections: Inspection[] } =
  JSON.parse(
    execFileSync(
      process.env.FLIGHTLENS_CARGO ?? "cargo",
      ["run", "--quiet", "-p", "flightlens-core", "--bin", "preview_fixture"],
      { encoding: "utf8" },
    ),
  );
const comparisons = [[], ["--zero-expo"], ["--vendor-missing-expo"]].map(
  (flags) =>
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
          "--explicit-only",
          ...flags,
        ],
        { encoding: "utf8" },
      ),
    ) as typeof fixture,
);
export async function aiFixture(
  page: Page,
  theme = "dark",
  enabled = true,
  compare = false,
) {
  await page.addInitScript(
    ({ fixture, comparisons, theme, enabled, compare }) => {
      const win = window as unknown as Record<string, unknown>;
      win.isTauri = true;
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      win.__aiCalls = [];
      win.__aiRequests = [];
      let status: LlmStatus = {
        settings: {
          provider: "custom",
          enabled,
          model: "synthetic-model",
          baseUrl: "http://localhost:11434",
          timeoutSeconds: 60,
        },
        hasKey: false,
        keyHint: null,
        sessionOnly: false,
        credentialProblem: null,
      };
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
            request?: SummaryRequest;
            sourceId?: string;
            configId?: string;
            settings?: LlmSettings;
            key?: string;
            sessionOnly?: boolean;
            rateProfile?: number;
          },
        ) => {
          if (command.startsWith("llm_"))
            (win.__aiCalls as string[]).push(command);
          if (args?.request)
            (win.__aiRequests as unknown[]).push({
              command,
              request: args.request,
            });
          if (command === "open_source")
            return comparisons[Number(args.sourceId)].artifact;
          if (command === "pending_sources" || command === "choose_files")
            return [];
          if (command === "restore_session")
            return {
              sources: compare
                ? comparisons.map((f, i) => ({
                    id: String(i),
                    label: f.artifact.document.title,
                  }))
                : [],
              unavailable: [],
            };
          if (command === "ingest_text") return fixture.artifact;
          if (command === "inspect_config")
            return (
              compare
                ? comparisons.find(
                    (f) => f.artifact.document.id === args.configId,
                  )!
                : fixture
            ).inspections[args.rateProfile ?? 0];
          if (command === "llm_settings") return status;
          if (command === "llm_save_settings") {
            status = { ...status, settings: args.settings! };
            return status;
          }
          if (command === "llm_save_key") {
            if (win.__keyFailure)
              throw "The OS credential store is unavailable or locked.";
            status = {
              ...status,
              hasKey: true,
              sessionOnly: args.sessionOnly!,
              keyHint: args.key!.slice(-4),
            };
            return status;
          }
          if (command === "llm_clear_key") {
            status = {
              ...status,
              hasKey: false,
              keyHint: null,
              sessionOnly: false,
            };
            return status;
          }
          if (command === "llm_preview" || command === "llm_test_preview") {
            const preview: LlmPreview = {
              systemPrompt:
                "Describe configuration, not flight behavior. Preserve unknown values.",
              userPrompt:
                command === "llm_test_preview"
                  ? "Connection test. No backup data is included."
                  : 'Summarize Backup A.\n{"motor_poles":null,"provenance":"unknown"}',
              payloadJson: '{"motor_poles":null,"provenance":"unknown"}',
              excluded: [{ key: "craft_name", line: 2, category: "identity" }],
              labelMap: [["Synthetic backup", "Backup A"]],
              bytes: 384,
              destination: "http://localhost:11434/v1/chat/completions",
              truncated: null,
              problems: (win.__previewProblems as string[] | undefined) ?? [],
            };
            if (args?.request?.kind === "diff") {
              preview.payloadJson = JSON.stringify({
                labels: args.request.slots.map((_, i) => `Backup ${"ABC"[i]}`),
                baseline: args.request.baseline,
                rows: args.request.rows,
              });
              preview.userPrompt = `Summarize the comparison.\n${preview.payloadJson}`;
              preview.labelMap = args.request.slots.map((slot, i) => [
                comparisons.find(
                  (f) => f.artifact.document.id === slot.configId,
                )!.artifact.document.title,
                `Backup ${"ABC"[i]}`,
              ]);
            }
            return preview;
          }
          if (command === "llm_summarize") {
            if (win.__holdSummary)
              await new Promise<void>((resolve) => {
                win.__resolveSummary = resolve;
              });
            if (win.__summaryError)
              throw "AI request timed out after 60 seconds.";
            const summary: LlmSummary = {
              text:
                args.request?.kind === "diff"
                  ? "Backups A and B differ in their declared rates. Unknown values remain unknown. These settings do not establish flight behavior."
                  : "Backup A contains partial configuration evidence. Motor poles are unknown.\n\n<script>Model output is plain text.</script>\n\nThis backup does not establish flight behavior.",
              provider: "custom",
              model: "synthetic-model",
              promptVersion: 1,
              generatedAt: "1789680000",
              cached: false,
            };
            return summary;
          }
          if (command === "llm_test_connection")
            return "Connection succeeded. No backup data was sent.";
          if (command === "llm_save_summary") return true;
          return 1;
        },
      };
      localStorage.setItem("flightlens.theme", theme);
    },
    { fixture, comparisons, theme, enabled, compare },
  );
  await page.goto("/");
  if (compare) {
    await page
      .getByRole("button", { name: "Compare backups", exact: true })
      .click();
    return;
  }
  await page
    .getByRole("button", { name: "Explore a synthetic example" })
    .click();
}
