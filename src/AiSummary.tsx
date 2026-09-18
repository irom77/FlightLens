import { useEffect, useRef, useState, type ReactNode } from "react";
import type {
  LlmPreview,
  LlmStatus,
  LlmSummary,
  SummaryRequest,
} from "./bindings/core";
import { api, desktopAvailable } from "./ipc/client";
import { AiPreview } from "./AiPreview";
import { useSelections } from "./stores/selections";
import {
  disclaimer,
  generatedTime,
  providers,
  summaryProblem,
} from "./aiSummarySettings";
import { message } from "./errorMessage";

export function AiResult({ summary }: { summary: LlmSummary }) {
  return (
    <>
      <pre className="ai-result-text">{summary.text}</pre>
      <p className="muted">
        {providers[summary.provider]} · {summary.model} · prompt v
        {summary.promptVersion} · {generatedTime(summary.generatedAt)}
        {summary.cached ? " · Cached" : ""}
      </p>
      <p className="notice">{disclaimer}</p>
    </>
  );
}
export function AiSummary({
  request,
  heading,
  status,
  openSettings,
  unavailable,
}: {
  request: SummaryRequest | null;
  unavailable?: string;
  heading?: ReactNode;
  status: LlmStatus | null;
  openSettings: () => void;
}) {
  const { summaries, setSummary: storeSetSummary } = useSelections();
  const requestKey = request ? JSON.stringify(request) : "";
  const summary = requestKey ? (summaries[requestKey] ?? null) : null;
  const [preview, setPreview] = useState<LlmPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const regenerate = useRef(false);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current++;
      void api.llmCancel().catch(() => {});
    },
    [],
  );
  const problem = unavailable || summaryProblem(desktopAvailable(), status);
  const run = async (action: (current: number) => Promise<void>) => {
    const current = epoch.current;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action(current);
    } catch (e) {
      if (current === epoch.current) setError(message(e));
    } finally {
      if (current === epoch.current) setBusy(false);
    }
  };
  const review = (fresh: boolean) =>
    void run(async (current) => {
      if (!request) return;
      regenerate.current = fresh;
      const p = await api.llmPreview(request);
      if (current === epoch.current) setPreview(p);
    });
  return (
    <section className="ai-summary" aria-label="AI summary">
      <div className="ai-summary-heading">
        {heading}
        <div className="ai-summary-controls">
          <div className="actions">
            <button
              disabled={!request || Boolean(problem) || busy}
              onClick={() => review(false)}
            >
              AI summary
            </button>
            <button className="text-button" onClick={openSettings}>
              AI settings
            </button>
          </div>
          {problem && <p className="muted">{problem}</p>}
        </div>
      </div>
      {busy && <p role="status">Preparing AI summary…</p>}
      {error && (
        <p role="alert" className="notice">
          {error}
        </p>
      )}
      {notice && <p role="status">{notice}</p>}
      {summary && (
        <div className="ai-result">
          <h2>AI summary</h2>
          <AiResult summary={summary} />
          <div className="actions">
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await navigator.clipboard.writeText(
                    `${summary.text}\n\n${providers[summary.provider]} · ${summary.model} · prompt v${summary.promptVersion} · ${generatedTime(summary.generatedAt)}\n\n${disclaimer}`,
                  );
                  setNotice("Summary copied with provenance and disclaimer.");
                })
              }
            >
              Copy
            </button>
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  if (!request) return;
                  const saved = await api.llmSaveSummary(request, summary);
                  setNotice(
                    saved
                      ? "Summary saved to a new Markdown file."
                      : "Save cancelled.",
                  );
                })
              }
            >
              Save as Markdown
            </button>
            <button
              disabled={busy || Boolean(problem)}
              onClick={() => review(true)}
            >
              Regenerate
            </button>
            <button onClick={() => storeSetSummary(requestKey, null)}>Dismiss</button>
          </div>
        </div>
      )}
      {preview && status && request && (
        <AiPreview
          preview={preview}
          settings={status.settings}
          onClose={() => {
            epoch.current++;
            setPreview(null);
            setBusy(false);
          }}
          send={async () => {
            const current = epoch.current;
            const result = await api.llmSummarize(request, regenerate.current);
            if (current === epoch.current) {
              storeSetSummary(requestKey, result);
              setPreview(null);
            }
          }}
        />
      )}
    </section>
  );
}
