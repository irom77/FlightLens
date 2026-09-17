import type { ComparisonDocument } from "./documentView";
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
import { AiSummary } from "./AiSummary";
import { summaryDiff } from "./summaryDiff";
import type { LlmStatus, SummaryRequest, Inspection } from "./bindings/core";
import { useProfiles, useSelections } from "./stores/selections";
import { api } from "./ipc/client";
import { Plot } from "./Plots";
import { ThreeParameterComparison } from "./ThreeParameterComparison";
import { ParameterComparison } from "./ParameterComparison";

function useRates(document: ComparisonDocument | undefined, profile: number) {
  const documentId = document?.id;
  const key = document ? `${document.id}:${profile}` : "";
  const [result, setResult] = useState<{
    key: string;
    inspection?: Inspection;
    error?: string;
  }>();
  useEffect(() => {
    if (!documentId) return;
    let current = true;
    api.inspect(documentId, profile).then(
      (inspection) => current && setResult({ key, inspection }),
      (error: unknown) => current && setResult({ key, error: String(error) }),
    );
    return () => {
      current = false;
    };
  }, [documentId, profile, key]);
  return result?.key === key ? result : undefined;
}

export function Comparison({
  documents,
  aiStatus,
  openAiSettings,
}: {
  documents: ComparisonDocument[];
  aiStatus: LlmStatus | null;
  openAiSettings: () => void;
}) {
  const { comparison, setComparison } = useSelections();
  const ids = comparison?.documents ?? [
    documents[0]?.id ?? "",
    documents[1]?.id ?? "",
    "",
  ];
  const [left = "", right = "", third = ""] = ids;
  const choose = (slot: number, value: string) => {
    const next = [left, right, third];
    next[slot] = value;
    setComparison({ documents: next, baseline: null });
  };
  const setLeft = (value: string) => choose(0, value);
  const setRight = (value: string) => choose(1, value);
  const setThird = (value: string) => choose(2, value);
  const c = documents.find((d) => d.id === third);
  const a = documents.find((d) => d.id === left);
  const b = documents.find((d) => d.id === right);
  return (
    <section className="content">
      <ComparisonSummary
        a={a}
        b={b}
        c={c}
        status={aiStatus}
        openSettings={openAiSettings}
      />
      <p>
        Calculated angular velocity (°/s) versus stick input, not measured
        flight motion. Choose two or three backups and a rate profile for each.
        Hover a plot to compare values at the same input. Throttle graphs show
        calculated throttle command (%) versus normalized throttle input (%).
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
            ["A", left, setLeft, [right, third]],
            ["B", right, setRight, [left, third]],
            ["C", third, setThird, [left, right]],
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
                <option key={d.id} value={d.id} disabled={other.includes(d.id)}>
                  {d.title} · {d.id.slice(0, 8)}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <RateComparison key={`${a?.id}:${b?.id}:${c?.id}`} a={a} b={b} c={c} />
    </section>
  );
}

function ComparisonSummary({
  a,
  b,
  c,
  status,
  openSettings,
}: {
  a?: ComparisonDocument;
  b?: ComparisonDocument;
  c?: ComparisonDocument;
  status: LlmStatus | null;
  openSettings: () => void;
}) {
  const pa = useProfiles(a);
  const pb = useProfiles(b);
  const pc = useProfiles(c);
  const selection = useSelections((s) => s.comparison);
  const index = [a?.id, b?.id, c?.id].indexOf(selection?.baseline ?? "");
  const baseline = c && index >= 0 ? index : null;
  const unavailable =
    !a || !b || a.id === b.id || (c && (c.id === a.id || c.id === b.id))
      ? "Select two or three different open backups to summarize."
      : c && baseline === null
        ? "Choose a baseline to summarize three backups."
        : undefined;
  const slots =
    a && b
      ? [
          { document: a, profiles: { rate: pa.rate, pid: pa.pid } },
          { document: b, profiles: { rate: pb.rate, pid: pb.pid } },
          ...(c
            ? [{ document: c, profiles: { rate: pc.rate, pid: pc.pid } }]
            : []),
        ]
      : [];
  const request: SummaryRequest | null = unavailable
    ? null
    : {
        kind: "diff",
        slots: slots.map(({ document, profiles }) => ({
          configId: document.id,
          rateProfile: profiles.rate,
          pidProfile: profiles.pid,
        })),
        baseline,
        rows: summaryDiff(slots, baseline),
      };
  return (
    <AiSummary
      key={JSON.stringify([request, slots.map((s) => s.document.hash), status])}
      request={request}
      status={status}
      openSettings={openSettings}
      unavailable={unavailable}
      heading={
        <div>
          <h1>Compare backups</h1>
          <p className="muted">
            AI summaries cover all non-equal comparison rows for the selected
            profiles, regardless of table search filters.
          </p>
        </div>
      }
    />
  );
}

function RateComparison({
  a,
  b,
  c,
}: {
  a?: ComparisonDocument;
  b?: ComparisonDocument;
  c?: ComparisonDocument;
}) {
  const { comparison, setComparison } = useSelections();
  const baselineIndex = [a?.id, b?.id, c?.id].indexOf(
    comparison?.baseline ?? "",
  );
  const baseline = baselineIndex < 0 ? "" : String(baselineIndex);
  const setBaseline = (value: string) =>
    setComparison({
      documents: [a?.id ?? "", b?.id ?? "", c?.id ?? ""],
      baseline:
        value === "" ? null : ([a?.id, b?.id, c?.id][Number(value)] ?? null),
    });
  const { rate: pa, setRate: setPa } = useProfiles(a);
  const { rate: pb, setRate: setPb } = useProfiles(b);
  const { rate: pc, setRate: setPc } = useProfiles(c);
  const rc = useRates(c, pc);
  const ra = useRates(a, pa);
  const rb = useRates(b, pb);
  const sources = [
    { side: "A", document: a, profile: pa, select: setPa, result: ra },
    { side: "B", document: b, profile: pb, select: setPb, result: rb },
    ...(c
      ? [{ side: "C", document: c, profile: pc, select: setPc, result: rc }]
      : []),
  ];
  const order = [
    Number(baseline),
    ...[0, 1, 2].filter((i) => i !== Number(baseline)),
  ];
  const selected = [a, b, c];
  const collectionDocuments = c ? order.map((i) => selected[i]) : selected;
  const collectionProps = {
    a: collectionDocuments[0]!,
    b: collectionDocuments[1]!,
    c: collectionDocuments[2],
    sides: order.map((i) => ["A", "B", "C"][i]) as [string, string, string],
  };
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
      {a &&
        b &&
        (c ? (
          <ThreeParameterComparison
            documents={[a, b, c]}
            rates={[pa, pb, pc]}
            baseline={baseline}
            setBaseline={setBaseline}
          />
        ) : (
          <ParameterComparison a={a} b={b} rateA={pa} rateB={pb} />
        ))}
      {a && b && (!c || baseline !== "") && (
        <>
          <FeatureComparison {...collectionProps} />
          <PortComparison {...collectionProps} />
          <ModeComparison {...collectionProps} />
          <RxRangeComparison {...collectionProps} />
          <RxFailComparison {...collectionProps} />
          <VtxTableComparison {...collectionProps} />
          <VtxActivationComparison {...collectionProps} />
          <AdjustmentRangeComparison {...collectionProps} />
          <CollectionComparison {...collectionProps} />
        </>
      )}
      {a && b && (
        <section aria-label="Throttle comparison">
          <h2>Throttle · command %</h2>
          <p>
            Static response from imported MID, EXPO and throttle limit settings.
            This does not predict motor output or thrust; runtime overrides are
            excluded.
          </p>
          <Plot
            throttle
            curves={sources.map(({ side, document, profile, result }) => {
              const curve = result?.inspection?.throttle;
              return {
                name: `${side}: ${document?.title} · Profile ${profile + 1}`,
                points: curve?.points ?? [],
                derivedInputs: curve?.derivedInputs ?? [],
                reason:
                  curve?.reason ??
                  (!curve
                    ? (result?.error ??
                      (result?.inspection
                        ? "Throttle unavailable for this backup/profile"
                        : "Loading…"))
                    : null),
              };
            })}
          />
        </section>
      )}
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
