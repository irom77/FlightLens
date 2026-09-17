import { useRef, useState } from "react";
import type { LlmPreview, LlmSettings } from "./bindings/core";
import { api } from "./ipc/client";
import { Modal } from "./Modal";
import { message } from "./errorMessage";
import { providers } from "./aiSummarySettings";

export function AiPreview({
  preview,
  settings,
  send,
  onClose,
}: {
  preview: LlmPreview;
  settings: LlmSettings;
  send: () => Promise<void>;
  onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const submitted = useRef(false);
  const close = () => {
    void api.llmCancel().catch(() => {});
    onClose();
  };
  return (
    <Modal labelledBy="ai-preview-title" onClose={close}>
      <div className="section-heading">
        <h2 id="ai-preview-title">Review AI request</h2>
        <button aria-label="Close AI preview" onClick={close}>
          ×
        </button>
      </div>
      <p>
        {providers[settings.provider]} · {settings.model}
      </p>
      <p className="ai-destination">Destination: {preview.destination}</p>
      <p>
        Only Send contacts this endpoint. A matching cached summary may be
        reused. {preview.bytes.toLocaleString()} payload bytes.
      </p>
      <div className="feedback-preview ai-preview">
        <h3>Exact system prompt</h3>
        <pre>{preview.systemPrompt}</pre>
        <h3>Exact user prompt</h3>
        <pre>{preview.userPrompt}</pre>
        <details>
          <summary>Structured digest JSON</summary>
          <pre>{preview.payloadJson}</pre>
        </details>
        <h3>Local label mapping (not sent)</h3>
        <ul>
          {preview.labelMap.map(([name, label], i) => (
            <li key={i}>
              {name} → {label}
            </li>
          ))}
        </ul>
        <h3>Excluded values</h3>
        <p>
          Raw text, filenames, source paths, unrecognized settings and
          diagnostic messages are excluded.
        </p>
        <ul>
          {preview.excluded.map((r, i) => (
            <li key={i}>
              {r.key} ({r.category}){r.line > 0 ? ` · line ${r.line}` : ""}
            </li>
          ))}
        </ul>
        {preview.truncated && (
          <p className="notice">
            Truncated: {preview.truncated.omitted} rows omitted (
            {preview.truncated.sections.join(", ")}).
          </p>
        )}
      </div>
      {preview.problems.map((problem) => (
        <p className="notice" key={problem}>
          {problem}
        </p>
      ))}
      {error && (
        <p role="alert" className="notice">
          {error} Close this dialog and review a new preview to retry.
        </p>
      )}
      {busy && (
        <p role="status">Waiting for the model… You can cancel this request.</p>
      )}
      <div className="actions">
        <button onClick={close}>Cancel</button>
        <button
          className="primary"
          disabled={busy || Boolean(error) || preview.problems.length > 0}
          onClick={() => {
            if (submitted.current) return;
            submitted.current = true;
            setBusy(true);
            void send()
              .catch((e) => setError(message(e)))
              .finally(() => setBusy(false));
          }}
        >
          Send to {providers[settings.provider]}
        </button>
      </div>
    </Modal>
  );
}
