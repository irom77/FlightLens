import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import { compareRxFails, type RxFail } from "./compareRxFails";

function RxFailValue({ value, side }: { value?: RxFail; side: string }) {
  if (!value)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = value.source;
  return (
    <>
      <div>Mode: {{ a: "Auto", h: "Hold", s: "Set" }[value.mode]}</div>
      {value.mode === "s" && <div>Stored value: {value.value} µs</div>}
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

export function RxFailComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareRxFails(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.channel)].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section
      aria-label="Receiver failsafe comparison"
      className="card parameter-table"
    >
      <h2>Receiver failsafe comparison</h2>
      <p>
        Final explicit rxfail declarations by channel index (0–17), on matching
        verified official 4.5.0–4.5.5 or 2025.12.1–2025.12.5 versions. Set
        values are normalized down to 25 µs steps. Resets and unverified syntax
        clear earlier knowledge; omitted declarations remain unknown. Vendor
        builds and cross-version comparisons are not certified. Equal
        declarations do not establish equal flight behavior or validate failsafe
        configuration.
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
        Hide equal receiver failsafes
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
                <RxFailValue value={row.a} side="A" />
              </td>
              <td>
                <RxFailValue value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No receiver failsafes match this filter.</p>}
    </section>
  );
}
