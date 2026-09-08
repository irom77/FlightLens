import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import { compareParameters, type ComparisonValue } from "./parameterComparison";

function Cell({
  value,
  document,
  side,
}: {
  value?: ComparisonValue;
  document: ConfigDocument;
  side: string;
}) {
  const declared = value?.declared;
  const derived = value?.derived;
  if (declared)
    return (
      <>
        <span>
          {declared.rawValue} {declared.unit}
        </span>
        <small>
          {" "}
          ·{" "}
          {declared.valid && declared.supported
            ? "Declared"
            : "Unknown · invalid or unsupported"}
        </small>
        <details>
          <summary className="source-value">
            {side}: Line {declared.line}
          </summary>
          <pre>
            {document.syntax.find((line) => line.line === declared.line)?.raw ??
              "Source line unavailable"}
          </pre>
        </details>
      </>
    );
  if (derived)
    return (
      <>
        {derived.rawValue}
        <small> · Derived · Betaflight {derived.sourceVersion} default</small>
      </>
    );
  return (
    <span className="unknown-badge">Unknown · not declared or derived</span>
  );
}

export function ParameterComparison({
  a,
  b,
  rateA,
  rateB,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
  rateA: number;
  rateB: number;
}) {
  const [pidA, setPidA] = useState(a.pidProfiles[0] ?? 0);
  const [pidB, setPidB] = useState(b.pidProfiles[0] ?? 0);
  const [query, setQuery] = useState("");
  const [differencesOnly, setDifferencesOnly] = useState(true);
  const rows = compareParameters(
    a,
    b,
    { pid: pidA, rate: rateA },
    { pid: pidB, rate: rateB },
  );
  const filtered = rows.filter(
    (row) =>
      row.key.includes(query.toLowerCase()) &&
      (!differencesOnly || row.status !== "Equal"),
  );
  return (
    <section aria-label="Parameter comparison" className="card parameter-table">
      <h2>Parameter comparison</h2>
      <p>
        Global settings and the selected PID and rate profiles. Unknown values
        are not changes. Different firmware versions are not yet certified for
        semantic comparison. Feature, port, mode and other CLI collection
        commands are not included.
      </p>
      <div className="profile-controls">
        {[
          { side: "A", document: a, pid: pidA, select: setPidA },
          { side: "B", document: b, pid: pidB, select: setPidB },
        ].map(({ side, document, pid, select }) => (
          <label className="profile" key={side}>
            PID profile {side}
            <select
              aria-label={`PID profile ${side}`}
              value={pid}
              onChange={(event) => select(Number(event.target.value))}
            >
              {(document.pidProfiles.length ? document.pidProfiles : [0]).map(
                (p) => (
                  <option value={p} key={p}>
                    Profile {p + 1}
                  </option>
                ),
              )}
            </select>
          </label>
        ))}
      </div>
      <input
        className="search"
        aria-label="Filter compared parameters"
        placeholder="Filter settings…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={differencesOnly}
          onChange={(event) => setDifferencesOnly(event.target.checked)}
        />{" "}
        Hide equal values
      </label>
      <p>
        {rows.filter((r) => r.status === "Changed").length} changed ·{" "}
        {rows.filter((r) => r.status === "Unknown").length} unknown ·{" "}
        {rows.filter((r) => r.status === "Not comparable").length} not
        comparable · {filtered.length} shown
      </p>
      <table>
        <thead>
          <tr>
            <th>Setting / scope</th>
            <th>A: {a.title}</th>
            <th>B: {b.title}</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr
              key={`${row.scope}:${row.key}`}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.key}</code>
                <small> · {row.scope}</small>
              </td>
              <td>
                <Cell value={row.a} document={a} side="A" />
              </td>
              <td>
                <Cell value={row.b} document={b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No settings match this filter.</p>}
    </section>
  );
}
