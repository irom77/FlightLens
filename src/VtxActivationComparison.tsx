import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import {
  compareVtxActivations,
  type VtxActivation,
} from "./compareVtxActivations";

function VtxActivationValue({
  value,
  side,
}: {
  value?: VtxActivation;
  side: string;
}) {
  if (!value)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = value.source;
  return (
    <>
      <div>AUX index: {value.aux}</div>
      <div>Band: {value.band || "Leave unchanged"}</div>
      <div>Channel: {value.channel || "Leave unchanged"}</div>
      <div>Power: {value.power || "Leave unchanged"}</div>
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

export function VtxActivationComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareVtxActivations(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.slot)].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section
      aria-label="VTX activation comparison"
      className="card parameter-table"
    >
      <h2>VTX activation comparison</h2>
      <p>
        Final explicit vtx declarations by slot (0–9), on matching verified
        official 4.5.0–4.5.5 versions. Ranges are clamped to 900–2100 µs and
        rounded down to 25 µs steps. Nonzero selectors require preceding table
        dimensions and bounds valid with or without table support;
        build-dependent cases remain unknown. Resets and unverified activation
        syntax clear earlier slot knowledge. Vendor, 2025.12 and cross-version
        comparisons are not certified. Equal declarations do not establish equal
        RF behavior or determine which overlapping activation wins.
      </p>
      <input
        className="search"
        aria-label="Filter compared VTX slots"
        placeholder="Filter VTX slots…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal VTX activations
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
                <VtxActivationValue value={row.a} side="A" />
              </td>
              <td>
                <VtxActivationValue value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No VTX activations match this filter.</p>}
    </section>
  );
}
