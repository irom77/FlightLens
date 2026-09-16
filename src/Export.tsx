import { useEffect, useState } from "react";
import type {
  DocumentView,
  ExportRequest,
  ValidatedSnippet,
} from "./bindings/core";
import { api } from "./ipc/client";
import { message } from "./errorMessage";
export function Export({
  document: d,
  pid,
  rate,
}: {
  document: DocumentView;
  pid: number;
  rate: number;
}) {
  const [groups, setGroups] = useState<string[]>(["rates"]);
  const [includeSave, setIncludeSave] = useState(false);
  const [validated, setValidated] = useState<{
    key: string;
    snippet: ValidatedSnippet;
  } | null>(null);
  const [status, setStatus] = useState("");
  const request: ExportRequest = {
    groups,
    pidProfile: pid,
    rateProfile: rate,
    destinationHeader: d.firmware.header ?? "",
    includeSave,
  };
  const requestKey = JSON.stringify(request);
  const snippet = validated?.key === requestKey ? validated.snippet : null;
  useEffect(() => {
    setValidated(null);
    setStatus("");
  }, [groups, pid, rate, includeSave]);
  return (
    <>
      <p>
        Export explicit settings from this donor. Firmware and target must match
        the original header; cross-target export is not certified.
      </p>
      <div className="card export-options">
        <small>DESTINATION · SOURCE IDENTITY</small>
        <code>{d.firmware.header ?? "Unknown — export unavailable"}</code>
        <div className="actions">
          {[
            ["rates", "Rates"],
            ["pids_filters", "PID / Filters"],
            ["modes", "Modes"],
            ["osd", "OSD"],
            ["serial", "Serial"],
            ["vtx", "VTX"],
          ].map(([key, label]) => (
            <label className="checkbox" key={key}>
              <input
                type="checkbox"
                checked={groups.includes(key)}
                onChange={(e) =>
                  setGroups(
                    e.target.checked
                      ? [...groups, key]
                      : groups.filter((g) => g !== key),
                  )
                }
              />
              {label}
            </label>
          ))}
        </div>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={includeSave}
            onChange={(e) => setIncludeSave(e.target.checked)}
          />
          Include device “save” command
        </label>
        <button
          className="primary"
          disabled={!d.firmware.packId || !groups.length}
          onClick={async () => {
            setStatus("");
            setValidated(null);
            try {
              setValidated({
                key: requestKey,
                snippet: await api.export(d.id, request),
              });
            } catch (e) {
              setStatus(message(e));
            }
          }}
        >
          Validate & preview
        </button>
      </div>
      {status && (
        <p role="status" className="notice">
          {status}
        </p>
      )}
      {snippet && (
        <>
          {snippet.additions.map((a) => (
            <p className="muted" key={a}>
              Dependency: {a}
            </p>
          ))}
          {snippet.warnings.map((w) => (
            <p className="notice" key={w}>
              {w}
            </p>
          ))}
          <pre className="snippet">{snippet.text}</pre>
          <div className="actions">
            <button
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(snippet.text);
                  setStatus("Copied validated snippet.");
                } catch {
                  setStatus(
                    "Clipboard unavailable. Use Save As or select the preview text.",
                  );
                }
              }}
            >
              Copy snippet
            </button>
            <button
              onClick={async () => {
                try {
                  setStatus(
                    (await api.save(d.id, request))
                      ? "Saved snippet to a new file."
                      : "Save cancelled.",
                  );
                } catch (e) {
                  setStatus(message(e));
                }
              }}
            >
              Save As…
            </button>
          </div>
        </>
      )}
    </>
  );
}
