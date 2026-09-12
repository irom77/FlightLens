import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import { compareRxRanges, type RxRange } from "./compareRxRanges";

function RxRangeValue({ value, side }: { value?: RxRange; side: string }) {
  if (!value)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = value.source;
  return (
    <>
      <div>Minimum: {value.min} µs</div>
      <div>Maximum: {value.max} µs</div>
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

export function RxRangeComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareRxRanges(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.channel)].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section
      aria-label="Receiver-range comparison"
      className="card parameter-table"
    >
      <h2>Receiver-range comparison</h2>
      <p>
        Final explicit rxrange endpoints by channel index (0–3), on matching
        verified official 4.5.0–4.5.5 or 2025.12.1–2025.12.5 versions. Resets
        and unverified syntax clear earlier knowledge; omitted ranges remain
        unknown. Vendor builds and cross-version comparisons are not certified.
        Matching endpoints do not establish receiver calibration or channel-map
        equivalence. Equal or reversed endpoints are preserved as declared,
        without claiming they are usable calibration.
      </p>
      <input
        className="search"
        aria-label="Filter compared receiver channels"
        placeholder="Filter receiver channels…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal receiver ranges
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
            <th>Channel index</th>
            <th>A: {a.title}</th>
            <th>B: {b.title}</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr
              key={row.channel}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.channel}</code>
              </td>
              <td>
                <RxRangeValue value={row.a} side="A" />
              </td>
              <td>
                <RxRangeValue value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No receiver ranges match this filter.</p>}
    </section>
  );
}
