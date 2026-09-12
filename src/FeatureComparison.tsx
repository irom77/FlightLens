import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import { compareFeatures } from "./compareFeatures";

function FeatureValue({
  document,
  name,
  value,
  side,
}: {
  document: ConfigDocument;
  name: string;
  value?: boolean;
  side: string;
}) {
  if (value === undefined)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = [...document.syntax]
    .reverse()
    .find(
      (line) => line.command.kind === "feature" && line.command.name === name,
    );
  return (
    <>
      {value ? "Enabled" : "Disabled"}
      <small> · Declared</small>
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

export function FeatureComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareFeatures(a, b);
  const filtered = rows.filter(
    (row) =>
      row.name.toLowerCase().includes(query.toLowerCase()) &&
      (!hideEqual || row.status !== "Equal"),
  );
  return (
    <section aria-label="Feature comparison" className="card parameter-table">
      <h2>Feature comparison</h2>
      <p>
        Final explicit feature declarations on the same firmware version.
        Missing declarations remain unknown. Cross-version features are not
        comparable. These states do not establish hardware support or whether a
        feature is active in flight.
      </p>
      <input
        className="search"
        aria-label="Filter compared features"
        placeholder="Filter features…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal features
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
            <th>Feature</th>
            <th>A: {a.title}</th>
            <th>B: {b.title}</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr
              key={row.name}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.name}</code>
              </td>
              <td>
                <FeatureValue
                  document={a}
                  name={row.name}
                  value={row.a}
                  side="A"
                />
              </td>
              <td>
                <FeatureValue
                  document={b}
                  name={row.name}
                  value={row.b}
                  side="B"
                />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No features match this filter.</p>}
    </section>
  );
}
