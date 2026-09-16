import type { ComparisonDocument } from "./documentView";
import { useState, type ReactNode } from "react";
import { compareThreeRows, type PairRow } from "./compareThreeRows";

export function ThreeCollectionTable<R extends PairRow<unknown>>({
  title,
  description,
  documents,
  sides,
  compare,
  rowKey,
  renderValue,
  textOnly,
}: {
  title: string;
  description: ReactNode;
  documents: [ComparisonDocument, ComparisonDocument, ComparisonDocument];
  sides: [string, string, string];
  compare: (a: ComparisonDocument, b: ComparisonDocument) => R[];
  rowKey: (row: R) => string;
  renderValue: (
    value: R["a"],
    document: ComparisonDocument,
    side: string,
    row: R,
  ) => ReactNode;
  textOnly: boolean;
}) {
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const [a, b, c] = documents;
  const rows = compareThreeRows<R>(
    [compare(a, b), compare(a, c), compare(b, c)],
    rowKey,
  );
  const filtered = rows.filter(
    (row) =>
      row.key.toLowerCase().includes(query.toLowerCase()) &&
      (!hideEqual || row.status !== "Equal"),
  );
  const label = (status: string) => {
    const result =
      status === "Left only"
        ? `${sides[1]} only`
        : status === "Right only"
          ? `${sides[2]} only`
          : status;
    return textOnly && status !== "Unknown"
      ? `${result === "Equal" ? "Matching" : result} text`
      : result;
  };
  return (
    <section
      aria-label={`Three-backup ${title.toLowerCase()}`}
      className="card parameter-table"
    >
      <h2>Three-backup {title.toLowerCase()}</h2>
      {description}
      <input
        className="search"
        aria-label={`Filter three-backup ${title.toLowerCase()}`}
        placeholder="Filter entries…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <label className="comparison-toggle">
        <input
          type="checkbox"
          checked={hideEqual}
          onChange={(e) => setHideEqual(e.target.checked)}
        />{" "}
        {textOnly ? "Hide matching text" : "Hide equal values"}
      </label>
      <p>
        {[
          "Left only",
          "Right only",
          "Same change",
          "Conflict",
          "Unknown",
          "Not comparable",
        ]
          .map(
            (status) =>
              `${rows.filter((row) => row.status === status).length} ${label(status).toLowerCase()}`,
          )
          .join(" · ")}{" "}
        · {filtered.length} shown
      </p>
      <table>
        <thead>
          <tr>
            <th>Entry</th>
            {documents.map((document, i) => (
              <th key={document.id}>
                {i === 0 ? "Baseline " : ""}
                {sides[i]}: {document.title}
              </th>
            ))}
            <th>
              {textOnly
                ? "Text differences from baseline"
                : "Changes from baseline"}
            </th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((row) => (
            <tr key={row.key}>
              <td>
                <code>{row.key}</code>
              </td>
              {documents.map((document, i) => (
                <td key={document.id}>
                  {renderValue(row.values[i], document, sides[i], row.row)}
                </td>
              ))}
              <td>
                {sides[1]}: {row.left} · {sides[2]}: {row.right}
              </td>
              <td>{label(row.status)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && <p>No entries match this filter.</p>}
    </section>
  );
}
