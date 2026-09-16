import type { DocumentView } from "./bindings/core";
import { Empty } from "./Empty";
export function PortsInspector({
  ports,
  source,
}: {
  ports: DocumentView["ports"];
  source: (n: number) => void;
}) {
  return (
    <>
      <div className="card">
        <table>
          <thead>
            <tr>
              <th>Port</th>
              <th>Functions</th>
              <th>MSP baud</th>
              <th>GPS baud</th>
              <th>Telemetry baud</th>
              <th>Blackbox baud</th>
            </tr>
          </thead>
          <tbody>
            {ports.map((p) => (
              <tr key={p.identifier}>
                <th>
                  <button
                    className="source-value"
                    onClick={() => source(p.line)}
                  >
                    {p.name}
                  </button>
                </th>
                <td>{p.functions.join(" + ") || "No functions"}</td>
                {p.baud.map((b, i) => (
                  <td key={i}>{b === 0 ? "AUTO" : b}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {!ports.length && (
          <Empty>No serial allocations are present in this artifact.</Empty>
        )}
      </div>
      <p className="notice">
        Multiple function bits can represent allowed sharing. Hardware
        availability and build-specific sharing validation remain unknown.
      </p>
    </>
  );
}
