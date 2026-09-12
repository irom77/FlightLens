import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import {
  compareAdjustmentRanges,
  type AdjustmentRange,
} from "./compareAdjustmentRanges";

function AdjustmentRangeValue({
  value,
  side,
}: {
  value?: AdjustmentRange;
  side: string;
}) {
  if (!value)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = value.source;
  return (
    <>
      <div>Range AUX index: {value.aux}</div>
      <div>Function ID: {value.functionId}</div>
      <div>Select AUX index: {value.selectAux}</div>
      <div>Center: {value.center}</div>
      <div>Scale: {value.scale}</div>
      <div>
        Stored range: {value.start}–{value.end} µs
      </div>
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

export function AdjustmentRangeComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareAdjustmentRanges(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.slot)].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section
      aria-label="Adjustment range comparison"
      className="card parameter-table"
    >
      <h2>Adjustment range comparison</h2>
      <p>
        Final explicit adjrange declarations by slot (0–29), on matching
        verified official 4.5.0–4.5.5 and 2025.12.1–2025.12.5 versions. Ranges
        are clamped to 900–2100 µs and rounded down to 25 µs steps. Omitted
        center and scale values are zero. Resets and unverified syntax clear
        earlier slot knowledge. Vendor and cross-version comparisons are not
        certified. Equal stored declarations do not establish equal in-flight
        effects or predict resulting gains, active profiles, or overlapping
        adjustments.
      </p>
      <input
        className="search"
        aria-label="Filter compared adjustment slots"
        placeholder="Filter adjustment slots…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal adjustment ranges
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
              key={row.slot}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.slot}</code>
              </td>
              <td>
                <AdjustmentRangeValue value={row.a} side="A" />
              </td>
              <td>
                <AdjustmentRangeValue value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No adjustment ranges match this filter.</p>}
    </section>
  );
}
