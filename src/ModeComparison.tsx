import { useState } from "react";
import type { ConfigDocument, Mode } from "./bindings/core";
import { compareModes } from "./compareModes";

function ModeValue({
  document,
  value,
  side,
}: {
  document: ConfigDocument;
  value?: Mode;
  side: string;
}) {
  if (!value)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = document.syntax.find((line) => line.line === value.line);
  return (
    <>
      <strong>{value.name}</strong>
      <small> · Declared</small>
      <div>Mode ID: {value.modeId}</div>
      <div>
        {value.channelAssigned
          ? `AUX ${value.channel + 1}`
          : `Unassigned channel (${value.channel})`}
      </div>
      <div>
        Range: {value.start}–{value.end}
      </div>
      <div>Logic: {value.logic ?? "Unknown · not declared"}</div>
      <div>Linked mode ID: {value.linked ?? "Unknown · not declared"}</div>
      {source && (
        <details>
          <summary className="source-value">
            {side}: Line {value.line}
          </summary>
          <pre>{source.raw}</pre>
        </details>
      )}
    </>
  );
}

export function ModeComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareModes(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.index), row.a?.name, row.b?.name].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section
      aria-label="Mode-assignment comparison"
      className="card parameter-table"
    >
      <h2>Mode-assignment comparison</h2>
      <p>
        Final explicit mode assignments, matched by slot index on the same
        firmware version. Mode IDs, channels, ranges, logic and linked mode IDs
        are compared. Missing slots or omitted logic/link fields remain unknown;
        cross-version assignments are not comparable. Slot differences do not
        establish different flight behavior, and matching declarations do not
        establish receiver or switch equivalence.
      </p>
      <input
        className="search"
        aria-label="Filter compared modes"
        placeholder="Filter modes…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal modes
      </label>
      <p>
        {rows.filter((row) => row.status === "Changed").length} changed ·{" "}
        {rows.filter((row) => row.status === "Unknown").length} unknown ·{" "}
        {rows.filter((row) => row.status === "Not comparable").length} not
        comparable · {filtered.length} shown
      </p>
      <table>
        <thead>
          <tr>
            <th>Slot index</th>
            <th>A: {a.title}</th>
            <th>B: {b.title}</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr
              key={row.index}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.index}</code>
              </td>
              <td>
                <ModeValue document={a} value={row.a} side="A" />
              </td>
              <td>
                <ModeValue document={b} value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No modes match this filter.</p>}
    </section>
  );
}
