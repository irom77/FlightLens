import { useState } from "react";
import type { ConfigDocument, SyntaxLine } from "./bindings/core";
import { compareCollections } from "./compareCollections";

function CollectionValue({
  value,
  side,
}: {
  value?: SyntaxLine[];
  side: string;
}) {
  if (!value)
    return <span className="unknown-badge">Unknown · not present</span>;
  return (
    <>
      {value.map((line) => (
        <details key={line.line}>
          <summary className="source-value">
            {side}: Line {line.line}
          </summary>
          <pre>{line.raw}</pre>
        </details>
      ))}
    </>
  );
}

export function CollectionComparison({
  a,
  b,
}: {
  a: ConfigDocument;
  b: ConfigDocument;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const rows = compareCollections(a, b);
  const filtered = rows.filter(
    (row) =>
      row.name.toLowerCase().includes(query.toLowerCase()) &&
      (!hideEqual || row.status !== "Matching text"),
  );
  return (
    <section
      aria-label="CLI collection text comparison"
      className="card parameter-table"
    >
      <h2>CLI collection text comparison</h2>
      <p>
        Source-text comparison of vtxtable, vtx, rxfail, rxrange, and adjrange.
        All recognized lines are grouped by command, retaining their order,
        duplicates, whitespace, and lines before resets. Missing groups remain
        unknown. Matching text does not establish valid commands, final
        settings, or equivalent behavior on either firmware version. Ordering
        between different command groups is not compared; use raw source for
        full context.
      </p>
      <input
        className="search"
        aria-label="Filter compared collections"
        placeholder="Filter collections…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(event) => setHideEqual(event.target.checked)}
        />{" "}
        Hide matching text
      </label>
      <p>
        {rows.filter((row) => row.status === "Different text").length} different
        text · {rows.filter((row) => row.status === "Unknown").length} unknown ·{" "}
        {filtered.length} shown
      </p>
      <table>
        <thead>
          <tr>
            <th>Command</th>
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
                row.status === "Different text"
                  ? "parameter-changed"
                  : undefined
              }
            >
              <td>
                <code>{row.name}</code>
              </td>
              <td>
                <CollectionValue value={row.a} side="A" />
              </td>
              <td>
                <CollectionValue value={row.b} side="B" />
              </td>
              <td>{row.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No collections match this filter.</p>}
    </section>
  );
}
