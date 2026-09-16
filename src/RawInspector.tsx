import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import type { DocumentView, Parameter, SyntaxLine } from "./bindings/core";
import { api } from "./ipc/client";
import { message } from "./errorMessage";
import { ParameterTable } from "./ParameterTable";
export function RawInspector({
  document: d,
  parameters,
  source,
  rawPage,
  setRawPage,
  line,
}: {
  document: DocumentView;
  parameters: Parameter[];
  source: (n: number) => void;
  rawPage: number;
  setRawPage: Dispatch<SetStateAction<number>>;
  line: number | null;
}) {
  const [result, setResult] = useState<{
    document: DocumentView;
    page: number;
    lines: SyntaxLine[];
    error?: string;
  } | null>(null);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let current = true;
    setResult(null);
    api.rawPage(d.id, rawPage * 500).then(
      (lines) => {
        if (current) setResult({ document: d, page: rawPage, lines });
      },
      (error) => {
        if (current)
          setResult({
            document: d,
            page: rawPage,
            lines: [],
            error: message(error),
          });
      },
    );
    return () => {
      current = false;
    };
  }, [d, rawPage, retry]);
  // Hide a previous document/page immediately, before the request effect runs.
  const loaded =
    result?.document === d && result.page === rawPage ? result : null;
  useEffect(() => {
    if (loaded && line)
      document
        .getElementById(`line-${line}`)
        ?.scrollIntoView({ block: "center" });
  }, [loaded, line]);
  return (
    <>
      <details>
        <summary>
          Normalized parameter table · {parameters.length} fields
        </summary>
        <ParameterTable rows={parameters} source={source} />
      </details>
      <details>
        <summary>Parser diagnostics · {d.diagnostics.length}</summary>
        {d.diagnostics.map((diag, i) => (
          <p className="diagnostic" key={i}>
            <b>{diag.severity}</b>{" "}
            {diag.line && (
              <button
                className="source-value"
                onClick={() => source(diag.line!)}
              >
                Line {diag.line}
              </button>
            )}{" "}
            {diag.message}
          </p>
        ))}
      </details>
      <div className="actions">
        <button
          disabled={rawPage === 0}
          onClick={() => setRawPage((p) => p - 1)}
        >
          Previous lines
        </button>
        <span className="muted">
          Lines {d.sourceEvidence.lineCount === 0 ? 0 : rawPage * 500 + 1}–
          {Math.min((rawPage + 1) * 500, d.sourceEvidence.lineCount)} of{" "}
          {d.sourceEvidence.lineCount}
        </span>
        <button
          disabled={(rawPage + 1) * 500 >= d.sourceEvidence.lineCount}
          onClick={() => setRawPage((p) => p + 1)}
        >
          Next lines
        </button>
      </div>
      {!loaded && <p role="status">Loading source lines…</p>}
      {loaded?.error && (
        <p role="alert" aria-label="Source loading error">
          {loaded.error}{" "}
          <button onClick={() => setRetry((n) => n + 1)}>
            Retry source lines
          </button>
        </p>
      )}
      <div className="raw-source" aria-busy={!loaded}>
        {(loaded?.lines ?? []).map((l) => (
          <div
            id={`line-${l.line}`}
            className={line === l.line ? "highlight-line" : ""}
            key={l.line}
          >
            <span>{l.line}</span>
            <code>{l.raw.replace(/\r?\n$/, "") || " "}</code>
            <small>
              {["unsupported", "malformed"].includes(l.command.kind)
                ? l.command.kind
                : ""}
            </small>
          </div>
        ))}
      </div>
    </>
  );
}
