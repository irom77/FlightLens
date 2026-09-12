import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import { compareVtxTables, type VtxTableValue } from "./compareVtxTables";

function TableValue({ value, side }: { value?: VtxTableValue; side: string }) {
  if (!value)
    return (
      <span className="unknown-badge">Unknown · missing or invalidated</span>
    );
  const source = value.source;
  return (
    <>
      <div>{value.value || "Empty array"}</div>
      <small>Declared</small>
      {source && (
        <details>
          <summary className="source-value">
            {side}: Line {source.line}
          </summary>
          <pre>{source.raw}</pre>
        </details>
      )}
    </>
  );
}

export function VtxTableComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareVtxTables(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.key)].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section aria-label="VTX table comparison" className="card parameter-table">
      <h2>VTX table comparison</h2>
      <p>
        Final explicit vtxtable declarations on matching verified official
        4.5.0–4.5.5 or 2025.12.1–2025.12.5 versions. Dimensions must be known
        before dependent entries. Resets, dimension changes and unverified
        syntax can leave entries unknown. Vendor builds and cross-version
        comparisons are not certified. Power values are stored numbers; matching
        declarations do not establish equal RF output.
      </p>
      <input
        className="search"
        aria-label="Filter compared VTX table entries"
        placeholder="Filter VTX table entries…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal VTX table entries
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
            <th>Table entry</th>
            <th>A: {a.title}</th>
            <th>B: {b.title}</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr
              key={row.key}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.key}</code>
              </td>
              <td>
                <TableValue value={row.a} side="A" />
              </td>
              <td>
                <TableValue value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No VTX table entries match this filter.</p>}
    </section>
  );
}
