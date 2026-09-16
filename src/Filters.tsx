import { useEffect, useState } from "react";
import type { DocumentView, Parameter, Point } from "./bindings/core";
import { api } from "./ipc/client";
import { message } from "./errorMessage";
import { Plot } from "./Plots";
import { ParameterTable } from "./ParameterTable";
export function Filters({
  document: d,
  parameters,
  source,
}: {
  document: DocumentView;
  parameters: Parameter[];
  source: (n: number) => void;
}) {
  const [sampleRate, setSampleRate] = useState("");
  const [selected, setSelected] = useState("");
  const [points, setPoints] = useState<Point[]>([]);
  const [error, setError] = useState("");
  const eligible = parameters.filter((p) =>
    [
      "gyro_lpf1_static_hz",
      "gyro_lpf2_static_hz",
      "dterm_lpf1_static_hz",
      "dterm_lpf2_static_hz",
    ].includes(p.key),
  );
  const parameterLines = parameters.map((p) => p.line).join(",");
  useEffect(() => {
    setPoints([]);
    setError("");
  }, [sampleRate, selected, parameterLines]);
  return (
    <>
      <div className="card filter-controls">
        <p>
          Static lowpass response requires the filter’s actual sample rate.
          Enter a measured or otherwise verified value.
        </p>
        <div className="actions">
          <label>
            Filter
            <select
              value={selected}
              onChange={(e) => setSelected(e.target.value)}
            >
              <option value="">Select an explicit filter</option>
              {eligible.map((p) => (
                <option key={p.line} value={p.line}>
                  {p.key}
                </option>
              ))}
            </select>
          </label>
          <label>
            Sample rate · Hz
            <input
              type="number"
              min="1"
              value={sampleRate}
              onChange={(e) => setSampleRate(e.target.value)}
            />
          </label>
          <button
            disabled={!selected || !sampleRate}
            onClick={async () => {
              const p = eligible.find((p) => String(p.line) === selected);
              if (!p) return;
              setError("");
              try {
                setPoints(
                  await api.filter(d.id, p.key, p.scope, Number(sampleRate)),
                );
              } catch (e) {
                setPoints([]);
                setError(message(e));
              }
            }}
          >
            Plot response
          </button>
        </div>
        {error && (
          <p role="alert" className="invalid">
            {error}
          </p>
        )}
        {points.length > 0 && (
          <Plot
            frequency
            curves={[
              {
                name: "Static lowpass",
                points,
                reason: null,
                derivedInputs: [],
              },
            ]}
          />
        )}
      </div>
      <p className="notice">
        Configured dynamic-notch and RPM-filter settings are listed below. Their
        actual in-flight frequencies require telemetry. The plot represents one
        static stage, not the complete filter chain.
      </p>
      <ParameterTable rows={parameters} source={source} />
    </>
  );
}
