import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AiResult } from "./AiSummary";
import {
  providerSettings,
  settingsProblem,
  summaryProblem,
} from "./aiSettings";
it("keeps summaries off and requires valid model, endpoint and timeout", () => {
  const settings = providerSettings("custom");
  expect(settings.enabled).toBe(false);
  expect(settingsProblem(settings)).toContain("model ID");
  settings.model = "local-model";
  expect(settingsProblem(settings)).toBe("");
  for (const baseUrl of [
    "http://remote.example",
    "https://user:secret@example.com",
    "https://example.com?secret=1",
    "file:///tmp/model",
  ])
    expect(settingsProblem({ ...settings, baseUrl })).not.toBe("");
  expect(settingsProblem({ ...settings, timeoutSeconds: 0 })).toContain(
    "Timeout",
  );
  expect(summaryProblem(false, null)).toContain("desktop");
  expect(
    summaryProblem(true, {
      settings: providerSettings("gemini"),
      hasKey: false,
      keyHint: null,
      sessionOnly: false,
      credentialProblem: null,
    }),
  ).toContain("off");
});
it("renders model output only as text with attribution and disclaimer", () => {
  const html = renderToStaticMarkup(
    <AiResult
      summary={{
        text: '<img src="https://invalid.test">',
        provider: "custom",
        model: "local",
        generatedAt: "1789680000",
        promptVersion: 1,
        cached: true,
      }}
    />,
  );
  expect(html).not.toContain("<img");
  expect(html).toContain("&lt;img");
  expect(html).toContain("not a certified finding");
  expect(html).toContain("Cached");
  expect(html).not.toContain("Invalid Date");
});
