import { useState } from "react";
import type { Parameter } from "./bindings/core";
import { value } from "./parameterValue";
import { Empty } from "./Empty";
export function ParameterTable({
  rows,
  source,
}: {
  rows: Parameter[];
  source: (line: number) => void;
}) {
  const [query, setQuery] = useState("");
  const filtered = rows.filter((p) => p.key.includes(query.toLowerCase()));
  const [limit, setLimit] = useState(100);
  return (
    <div className="card parameter-table">
      <input
        className="search"
        aria-label="Filter parameters"
        placeholder="Filter settings…"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setLimit(100);
        }}
      />
      <table>
        <thead>
          <tr>
            <th>Setting</th>
            <th>Value</th>
            <th>Scope</th>
            <th>Origin</th>
          </tr>
        </thead>
        <tbody>
          {filtered.slice(0, limit).map((p) => (
            <tr key={`${p.scope.kind}-${p.line}-${p.key}`}>
              <td>
                <code>{p.key}</code>
              </td>
              <td className={p.valid ? "" : "invalid"}>
                {value(p)} {p.unit}
              </td>
              <td>
                {p.scope.kind}
                {"index" in p.scope ? ` ${p.scope.index}` : ""}
              </td>
              <td>
                <button className="source-value" onClick={() => source(p.line)}>
                  Line {p.line}
                </button>
                {!p.supported && <small> · unsupported</small>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {!filtered.length && (
        <Empty>
          No matching explicit settings. Missing values remain unknown.
        </Empty>
      )}
      {filtered.length > limit && (
        <button onClick={() => setLimit((n) => n + 100)}>
          Show next 100 settings
        </button>
      )}
    </div>
  );
}
