import { useEffect, useState } from "react";
import type {
  FeedbackReport,
  FeedbackRequest,
  ReportKind,
} from "./bindings/core";
import { api } from "./ipc/client";

const message = (e: unknown) =>
  typeof e === "string"
    ? e
    : e instanceof Error
      ? e.message
      : "The report could not be prepared.";

const categories: Record<string, string> = {
  identity: "name",
  link_secret: "radio link identity",
};

/**
 * Collects a report and hands it to the reporter, who files it themselves.
 *
 * FlightLens transmits nothing: the issue is opened prefilled in the browser
 * under the reporter's own GitHub account, and an attached backup travels on
 * the clipboard. Everything that leaves the app is shown here first, because a
 * report published by hand cannot be recalled.
 */
export function Feedback({
  configId,
  documentTitle,
  onClose,
}: {
  configId: string | null;
  documentTitle: string | null;
  onClose: () => void;
}) {
  const [kind, setKind] = useState<ReportKind>("bug");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [includeConfig, setIncludeConfig] = useState(false);
  const [prepared, setPrepared] = useState<{
    key: string;
    report: FeedbackReport;
  } | null>(null);
  const [filing, setFiling] = useState(false);
  const [incomplete, setIncomplete] = useState("");
  const [error, setError] = useState("");
  const [filed, setFiled] = useState(false);
  // The app version and platform are stamped by the desktop shell, which knows
  // what is actually running; whatever is sent from here is discarded.
  const request: FeedbackRequest = {
    kind,
    subject,
    body,
    includeConfig,
    appVersion: "",
    platform: "",
  };
  const requestKey = JSON.stringify([configId, request]);
  const report = prepared?.key === requestKey ? prepared.report : null;
  useEffect(() => {
    let current = true;
    api
      .feedbackReport(configId, request)
      .then((r) => {
        if (!current) return;
        setPrepared({ key: requestKey, report: r });
        setIncomplete("");
      })
      .catch((e) => {
        if (!current) return;
        setPrepared(null);
        setIncomplete(message(e));
      });
    return () => {
      current = false;
    };
  }, [configId, kind, subject, body, includeConfig]);
  const file = async () => {
    if (!report || filing) return;
    setFiling(true);
    setError("");
    if (report.clipboard !== null) {
      try {
        await navigator.clipboard.writeText(report.clipboard);
      } catch {
        setError(
          "The configuration could not be copied, so the issue was not opened. Allow clipboard access, or file the report without a configuration.",
        );
        setFiling(false);
        return;
      }
    }
    try {
      await api.fileFeedback(configId, request);
      setFiled(true);
    } catch (e) {
      setError(message(e));
    } finally {
      setFiling(false);
    }
  };
  return (
    <div className="modal-backdrop">
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="feedback-title"
      >
        <div className="section-heading">
          <h2 id="feedback-title">Send feedback</h2>
          <button aria-label="Close feedback dialog" onClick={onClose}>
            ×
          </button>
        </div>
        {filed ? (
          <>
            <p>
              The prefilled issue is open in your browser. Review it, then
              submit it from your own GitHub account.
              {report?.clipboard &&
                " The redacted configuration is on your clipboard; paste it into the issue where the body asks for it."}
            </p>
            <div className="actions">
              <button className="primary" onClick={onClose}>
                Done
              </button>
            </div>
          </>
        ) : (
          <>
            <p>Submitting feedback requires a GitHub account.</p>
            <p>
              FlightLens does not send anything. The issue opens prefilled in
              your browser and you file it yourself; an attached configuration
              is copied to your clipboard for you to paste.
            </p>
            <fieldset className="feedback-kind" disabled={filing}>
              <legend>What is this?</legend>
              <label>
                <input
                  type="radio"
                  name="feedback-kind"
                  checked={kind === "bug"}
                  onChange={() => setKind("bug")}
                />
                Something is wrong
              </label>
              <label>
                <input
                  type="radio"
                  name="feedback-kind"
                  checked={kind === "feature"}
                  onChange={() => setKind("feature")}
                />
                Request a feature
              </label>
            </fieldset>
            <label>
              Subject
              <input
                disabled={filing}
                autoFocus
                value={subject}
                onChange={(e) => setSubject(e.target.value)}
                maxLength={120}
                placeholder="Rates preview stays blank"
              />
            </label>
            <label className="feedback-description">
              Description
              <textarea
                disabled={filing}
                value={body}
                onChange={(e) => setBody(e.target.value)}
                maxLength={2000}
                placeholder="What you did, what you expected, and what happened instead."
              />
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={includeConfig}
                disabled={!configId || filing}
                onChange={(e) => setIncludeConfig(e.target.checked)}
              />
              Attach the open configuration
              {configId ? ` (${documentTitle})` : " (no backup is open)"}
            </label>
            {report && (
              <details className="feedback-preview" open={includeConfig}>
                <summary>Review what leaves this app</summary>
                <h3>Issue body</h3>
                <pre>{report.issueBody}</pre>
                {report.clipboard !== null && (
                  <>
                    <h3>
                      Configuration copied to your clipboard
                      {report.removed.length > 0 &&
                        `, with ${report.removed.length} value(s) removed`}
                    </h3>
                    {report.removed.length > 0 && (
                      <ul className="feedback-removed">
                        {report.removed.map((r) => (
                          <li key={`${r.line}:${r.key}`}>
                            Line {r.line}: {r.key} ({categories[r.category]})
                          </li>
                        ))}
                      </ul>
                    )}
                    {report.oversized && (
                      <p className="notice">
                        This configuration and report exceed the GitHub issue
                        body limit. Save it from your clipboard and attach it to
                        the issue as a file instead of pasting it.
                      </p>
                    )}
                    <pre>{report.clipboard}</pre>
                  </>
                )}
              </details>
            )}
            {error && (
              <p className="notice" role="alert">
                {error}
              </p>
            )}
            <div className="actions">
              <span className="feedback-status">{incomplete}</span>
              <button onClick={onClose}>Cancel</button>
              <button
                className="primary"
                disabled={!report || filing}
                onClick={() => void file()}
              >
                Continue to GitHub
              </button>
            </div>
          </>
        )}
      </section>
    </div>
  );
}
