import type { Derived } from "./bindings/core";
/// Values no source line declares. Kept out of the parameter table on purpose:
/// these have no line to jump to, and must never read as though they do.
export function DerivedTable({ rows }: { rows: Derived[] }) {
  if (!rows.length) return null;
  return (
    <div className="card parameter-table">
      <div className="card-heading">
        Read back from firmware defaults <span>{rows.length} settings</span>
      </div>
      <table>
        <thead>
          <tr>
            <th>Setting</th>
            <th>Value</th>
            <th>Origin</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((v) => (
            <tr key={v.key}>
              <td>{v.key}</td>
              <td>{v.rawValue}</td>
              <td className="muted">
                Betaflight {v.sourceVersion} default · not declared, not
                exported
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
