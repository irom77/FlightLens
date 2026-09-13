import { useProfiles } from "./stores/selections";
import { useState } from "react";
import type { ConfigDocument } from "./bindings/core";
import { compareThreeParameters } from "./compareThreeParameters";
import { ParameterValueCell } from "./ParameterComparison";

const sides = ["A", "B", "C"];

export function ThreeParameterComparison({
  documents,
  rates,
  baseline,
  setBaseline,
}: {
  documents: [ConfigDocument, ConfigDocument, ConfigDocument];
  rates: [number, number, number];
  baseline: string;
  setBaseline: (value: string) => void;
}) {
  const selected = [
    useProfiles(documents[0]),
    useProfiles(documents[1]),
    useProfiles(documents[2]),
  ];
  const pids = selected.map((p) => p.pid);
  const [query, setQuery] = useState("");
  const [hideEqual, setHideEqual] = useState(true);
  const order = [
    Number(baseline),
    ...[0, 1, 2].filter((i) => i !== Number(baseline)),
  ];
  const ordered = order.map((i) => documents[i]) as typeof documents;
  const profiles = order.map((i) => ({
    pid: pids[i],
    rate: rates[i],
  })) as Parameters<typeof compareThreeParameters>[1];
  const rows = baseline === "" ? [] : compareThreeParameters(ordered, profiles);
  const filtered = rows.filter(
    (row) =>
      row.key.includes(query.toLowerCase()) &&
      (!hideEqual || row.status !== "Equal"),
  );
  const resultLabel = (status: string) =>
    status === "Left only"
      ? `${sides[order[1]]} only`
      : status === "Right only"
        ? `${sides[order[2]]} only`
        : status;
  return (
    <section
      aria-label="Three-backup parameter comparison"
      className="card parameter-table"
    >
      <h2>Three-backup parameter comparison</h2>
      <p>
        Select a baseline to distinguish one-sided changes, the same change in
        both backups, and conflicting changes. Unknown or unsupported
        comparisons remain explicit. All backups are read-only.
      </p>
      <label className="profile">
        Baseline
        <select
          aria-label="Comparison baseline"
          value={baseline}
          onChange={(e) => setBaseline(e.target.value)}
        >
          <option value="">Select a baseline</option>
          {documents.map((d, i) => (
            <option key={d.id} value={i}>
              {sides[i]}: {d.title}
            </option>
          ))}
        </select>
      </label>
      <div className="profile-controls">
        {documents.map((d, i) => (
          <label className="profile" key={d.id}>
            PID profile {sides[i]}
            <select
              aria-label={`PID profile ${sides[i]}`}
              value={pids[i]}
              onChange={(e) => selected[i].setPid(Number(e.target.value))}
            >
              {(d.pidProfiles.length ? d.pidProfiles : [0]).map((p) => (
                <option key={p} value={p}>
                  Profile {p + 1}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      {baseline === "" ? (
        <p role="status">Choose a baseline to show parameter differences.</p>
      ) : (
        <>
          <input
            className="search"
            aria-label="Filter compared parameters"
            placeholder="Filter settings…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <label className="comparison-toggle">
            <input
              type="checkbox"
              checked={hideEqual}
              onChange={(e) => setHideEqual(e.target.checked)}
            />{" "}
            Hide equal values
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
                  `${rows.filter((row) => row.status === status).length} ${resultLabel(status).toLowerCase()}`,
              )
              .join(" · ")}{" "}
            · {filtered.length} shown
          </p>
          <table>
            <thead>
              <tr>
                <th>Setting / scope</th>
                {order.map((i, j) => (
                  <th key={i}>
                    {j === 0 ? "Baseline " : ""}
                    {sides[i]}: {documents[i].title}
                  </th>
                ))}
                <th>Changes from baseline</th>
                <th>Result</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((row) => (
                <tr key={`${row.scope}:${row.key}`}>
                  <td>
                    <code>{row.key}</code>
                    <small> · {row.scope}</small>
                  </td>
                  {order.map((i, j) => (
                    <td key={i}>
                      <ParameterValueCell
                        value={row.values[j]}
                        document={documents[i]}
                        side={sides[i]}
                      />
                    </td>
                  ))}
                  <td>
                    {sides[order[1]]}: {row.left} · {sides[order[2]]}:{" "}
                    {row.right}
                  </td>
                  <td>{resultLabel(row.status)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {!filtered.length && <p>No settings match this filter.</p>}
        </>
      )}
    </section>
  );
}
