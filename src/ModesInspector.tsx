import type { DocumentView } from "./bindings/core";
import { Empty } from "./Empty";
export function ModesInspector({
  modes,
  source,
}: {
  modes: DocumentView["modes"];
  source: (n: number) => void;
}) {
  return (
    <>
      <div className="card">
        {modes.map((m) => (
          <div className="mode-row" key={m.index}>
            <div>
              <button className="source-value" onClick={() => source(m.line)}>
                {m.name}
              </button>
              <small>
                Slot {m.index} ·{" "}
                {m.channelAssigned
                  ? `AUX ${m.channel + 1}`
                  : "no channel assigned"}{" "}
                ·{" "}
                {m.logic === null
                  ? "logic unspecified"
                  : m.logic === 0
                    ? "OR"
                    : "AND"}
                {m.linked ? ` · linked ID ${m.linked}` : ""}
              </small>
            </div>
            <div className="range-track">
              <span
                style={{
                  left: `${(m.start - 900) / 12}%`,
                  width: `${(m.end - m.start) / 12}%`,
                }}
              />
              <small>
                {m.start} — {m.end} μs
                {m.start === m.end ? " · inactive" : ""}
              </small>
            </div>
          </div>
        ))}
        {!modes.length && (
          <Empty>No explicit mode ranges. Omitted ranges remain unknown.</Empty>
        )}
      </div>
      <p className="notice">
        AUX numbering is displayed from 1; CLI channels are zero-based. Inactive
        ranges and linked-mode relationships are preserved. A mode with no
        channel assigned is shown as written; Betaflight discards such a row if
        it is pasted back.
      </p>
    </>
  );
}
