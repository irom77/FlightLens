import { AdjustmentRangeComparison } from "./AdjustmentRangeComparison";
import { VtxActivationComparison } from "./VtxActivationComparison";
import { VtxTableComparison } from "./VtxTableComparison";
import { RxFailComparison } from "./RxFailComparison";
import { RxRangeComparison } from "./RxRangeComparison";
import { CollectionComparison } from "./CollectionComparison";
import { ModeComparison } from "./ModeComparison";
import { PortComparison } from "./PortComparison";
import { FeatureComparison } from "./FeatureComparison";
import { useEffect, useState } from "react";
import type { ConfigDocument, Inspection } from "./bindings/core";
import { api } from "./ipc/client";
import { Plot } from "./Plots";
import { ParameterComparison } from "./ParameterComparison";

function useRates(document: ConfigDocument | undefined, profile: number) {
  const key = document ? `${document.id}:${profile}` : "";
  const [result, setResult] = useState<{
    key: string;
    inspection?: Inspection;
    error?: string;
  }>();
  useEffect(() => {
    if (!document) return;
    let current = true;
    api.inspect(document.id, profile).then(
      (inspection) => current && setResult({ key, inspection }),
      (error: unknown) => current && setResult({ key, error: String(error) }),
    );
    return () => {
      current = false;
    };
  }, [document?.id, profile, key]);
  return result?.key === key ? result : undefined;
}

export function Comparison({ documents }: { documents: ConfigDocument[] }) {
  const [left, setLeft] = useState(documents[0]?.id ?? "");
  const [right, setRight] = useState(documents[1]?.id ?? "");
  const a = documents.find((d) => d.id === left);
  const b = documents.find((d) => d.id === right);
  return (
    <section className="content">
      <h1>Compare backups</h1>
      <p>
        Calculated angular velocity (°/s) versus stick input, not measured
        flight motion. Choose two backups and a rate profile for each. Hover a
        plot to compare values at the same stick position.
      </p>
      {documents.length < 2 && (
        <p role="status">
          Open at least two different backups to compare. Identical file
          contents share one document.
        </p>
      )}
      <div className="profile-controls">
        {(
          [
            ["A", left, setLeft, right],
            ["B", right, setRight, left],
          ] as const
        ).map(([side, id, select, other]) => (
          <label className="profile" key={side}>
            Backup {side}
            <select
              aria-label={`Backup ${side}`}
              value={documents.some((d) => d.id === id) ? id : ""}
              onChange={(e) => select(e.target.value)}
            >
              <option value="">Select a backup</option>
              {documents.map((d) => (
                <option key={d.id} value={d.id} disabled={d.id === other}>
                  {d.title} · {d.id.slice(0, 8)}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <RateComparison key={`${a?.id}:${b?.id}`} a={a} b={b} />
    </section>
  );
}

function RateComparison({ a, b }: { a?: ConfigDocument; b?: ConfigDocument }) {
  const [pa, setPa] = useState(a?.rateProfiles[0] ?? 0);
  const [pb, setPb] = useState(b?.rateProfiles[0] ?? 0);
  const ra = useRates(a, pa);
  const rb = useRates(b, pb);
  const sources = [
    { side: "A", document: a, profile: pa, select: setPa, result: ra },
    { side: "B", document: b, profile: pb, select: setPb, result: rb },
  ];
  return (
    <>
      <div className="profile-controls">
        {sources.map(
          ({ side, document, profile, select, result }) =>
            document && (
              <div key={side}>
                <label className="profile">
                  Rate profile {side}
                  <select
                    aria-label={`Rate profile ${side}`}
                    value={profile}
                    onChange={(e) => select(Number(e.target.value))}
                  >
                    {(document.rateProfiles.length
                      ? document.rateProfiles
                      : [0]
                    ).map((p) => (
                      <option key={p} value={p}>
                        Profile {p + 1}
                      </option>
                    ))}
                  </select>
                </label>
                <p>
                  {side}: {document.title} · {document.firmware.family}{" "}
                  {document.firmware.version}
                </p>
                {!result && <p role="status">Loading {side}…</p>}
                {result?.error && (
                  <p role="alert">
                    {side}: {result.error}
                  </p>
                )}
              </div>
            ),
        )}
      </div>
      {a && b && <ParameterComparison a={a} b={b} rateA={pa} rateB={pb} />}
      {a && b && <FeatureComparison a={a} b={b} />}
      {a && b && <PortComparison a={a} b={b} />}
      {a && b && <ModeComparison a={a} b={b} />}
      {a && b && <RxRangeComparison a={a} b={b} />}
      {a && b && <RxFailComparison a={a} b={b} />}
      {a && b && <VtxTableComparison a={a} b={b} />}
      {a && b && <VtxActivationComparison a={a} b={b} />}
      {a && b && <AdjustmentRangeComparison a={a} b={b} />}
      {a && b && <CollectionComparison a={a} b={b} />}
      {a && b ? (
        ["roll", "pitch", "yaw"].map((axis) => (
          <section key={axis} aria-label={`${axis} comparison`}>
            <h2>{axis[0].toUpperCase() + axis.slice(1)} · °/s</h2>
            <Plot
              curves={sources.map(({ side, document, profile, result }) => {
                const curve = result?.inspection?.rates.find(
                  (c) => c.name.toLowerCase() === axis,
                );
                return {
                  name: `${side}: ${document?.title} · Profile ${profile + 1}`,
                  points: curve?.points ?? [],
                  derivedInputs: curve?.derivedInputs ?? [],
                  reason:
                    curve?.reason ??
                    (!curve
                      ? (result?.error ??
                        (result?.inspection
                          ? "Angular velocity unavailable for this backup/profile"
                          : "Loading…"))
                      : null),
                };
              })}
            />
            {sources.map(({ side, result }) => {
              const inputs = result?.inspection?.rates.find(
                (c) => c.name.toLowerCase() === axis,
              )?.derivedInputs;
              return inputs?.length ? (
                <p className="muted" key={side}>
                  {side}: uses firmware-derived defaults for {inputs.join(", ")}
                </p>
              ) : null;
            })}
          </section>
        ))
      ) : (
        <p role="status">
          Select two available backups. If a selected document was closed,
          choose a replacement.
        </p>
      )}
    </>
  );
}
