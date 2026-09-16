import { sourceLine } from "./documentView";
import type { ComparisonDocument } from "./documentView";
import { ThreeCollectionTable } from "./ThreeCollectionTable";
import { useState } from "react";
import type { Port } from "./bindings/core";
import { comparePorts } from "./comparePorts";

function PortValue({
  document,
  value,
  side,
}: {
  document: ComparisonDocument;
  value?: Port;
  side: string;
}) {
  if (!value)
    return <span className="unknown-badge">Unknown · not declared</span>;
  const source = sourceLine(document, value.line);
  return (
    <>
      <strong>{value.name}</strong>
      <small> · Declared</small>
      <div>
        {value.functions.join(" + ") || "No functions"} · Mask {value.mask}
      </div>
      {value.baud.map((baud, index) => (
        <div key={index}>
          {["MSP", "GPS", "Telemetry", "Blackbox"][index]} baud:{" "}
          {baud === 0 ? "AUTO (0)" : baud}
        </div>
      ))}
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

export function PortComparison({
  a,
  b,
  c,
  sides,
}: {
  a: ComparisonDocument;
  b: ComparisonDocument;
  c?: ComparisonDocument;
  sides?: [string, string, string];
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const description = (
    <p>
      Final explicit serial allocations, matched by port identifier on the same
      firmware version. Function masks and all four baud settings are compared.
      Missing ports remain unknown; cross-version allocations are not
      comparable. Matching allocations do not establish identical board wiring
      or resource assignments.
    </p>
  );
  const rows = comparePorts(a, b);
  const filtered = rows.filter(
    (row) =>
      [String(row.identifier), row.a?.name, row.b?.name].some((name) =>
        name?.toLowerCase().includes(query.toLowerCase()),
      ) &&
      (!hideEqual || row.status !== "Equal"),
  );
  if (c && sides)
    return (
      <ThreeCollectionTable
        title="Serial-port comparison"
        description={description}
        documents={[a, b, c]}
        sides={sides}
        compare={comparePorts}
        rowKey={(row) => String(row.identifier)}
        renderValue={(value, document, side) => (
          <PortValue value={value} side={side} document={document} />
        )}
        textOnly={false}
      />
    );
  return (
    <section
      aria-label="Serial-port comparison"
      className="card parameter-table"
    >
      <h2>Serial-port comparison</h2>
      {description}
      <input
        className="search"
        aria-label="Filter compared ports"
        placeholder="Filter ports…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide equal ports
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
            <th>Port identifier</th>
            <th>A: {a.title}</th>
            <th>B: {b.title}</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr
              key={row.identifier}
              className={
                row.status === "Changed" ? "parameter-changed" : undefined
              }
            >
              <td>
                <code>{row.identifier}</code>
              </td>
              <td>
                <PortValue document={a} value={row.a} side="A" />
              </td>
              <td>
                <PortValue document={b} value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No ports match this filter.</p>}
    </section>
  );
}
