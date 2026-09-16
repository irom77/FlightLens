import { useEffect, useState } from "react";
import type { DocumentView, Inspection, Parameter } from "./bindings/core";
import { api } from "./ipc/client";
import { message } from "./errorMessage";
import { tabs, useWorkspace } from "./stores/workspace";
import { useProfiles } from "./stores/selections";
import { populatedProfiles } from "./inspectionProfiles";
import { RatesInspector } from "./RatesInspector";
import { PIDInspector } from "./PIDInspector";
import { PortsInspector } from "./PortsInspector";
import { ModesInspector } from "./ModesInspector";
import { RawInspector } from "./RawInspector";
import { AuditInspector } from "./AuditInspector";
import { Filters } from "./Filters";
import { Osd } from "./Osd";
import { Export } from "./Export";

const empty: Inspection = {
  rates: [],
  throttle: {
    name: "throttle",
    points: [],
    reason: "Loading throttle preview",
    derivedInputs: [],
  },
  osd: [],
  audits: [],
};
export function Inspector({
  document: d,
  onError,
  reload,
}: {
  document: DocumentView;
  onError: (e: string) => void;
  reload: () => void;
}) {
  const { tab, setTab } = useWorkspace();
  // Keep original CLI numbers, but skip sections with no inspection data.
  const pidProfiles = populatedProfiles(d, "pid");
  const rateProfiles = populatedProfiles(d, "rate");
  const { pid, rate, setPid, setRate } = useProfiles(d);
  if (!pidProfiles.includes(pid)) pidProfiles.push(pid);
  if (!rateProfiles.includes(rate)) rateProfiles.push(rate);
  const [inspection, setInspection] = useState<Inspection>(empty);
  const [rateInspections, setRateInspections] = useState<
    Record<number, Inspection>
  >({});
  const [line, setLine] = useState<number | null>(null);
  const [rawPage, setRawPage] = useState(0);
  useEffect(() => {
    let current = true;
    setInspection(empty);
    setRateInspections({});
    Promise.all(
      (d.rateProfiles.length ? d.rateProfiles : [rate]).map(
        async (profile) => [profile, await api.inspect(d.id, profile)] as const,
      ),
    )
      .then((entries) => {
        if (!current) return;
        const all = Object.fromEntries(entries);
        setRateInspections(all);
        setInspection(all[rate] ?? empty);
      })
      .catch((e) => {
        if (current) onError(message(e));
      });
    return () => {
      current = false;
    };
  }, [d.id, rate, d.rateProfiles, onError]);
  const source = (n: number) => {
    setLine(n);
    setRawPage(Math.floor((n - 1) / 500));
    setTab("Raw");
  };
  const parameters = Object.values(d.parameters).filter(
    (p): p is Parameter => p !== undefined,
  );
  const current = parameters.filter(
    (p) =>
      p.scope.kind === "global" ||
      (p.scope.kind === "pid" && p.scope.index === pid) ||
      (p.scope.kind === "rate" && p.scope.index === rate),
  );
  const profile = (
    kind: "PID" | "Rate",
    values: number[],
    selected: number,
    change: (n: number) => void,
  ) => (
    <label className="profile">
      {kind} profile
      <select value={selected} onChange={(e) => change(Number(e.target.value))}>
        {(values.length ? values : [0]).map((n) => (
          <option key={n} value={n}>
            {values.length ? `${n + 1} · CLI ${n}` : "Unknown"}
          </option>
        ))}
      </select>
    </label>
  );
  return (
    <>
      <section className="document-heading">
        <div>
          <span className="eyebrow">CONFIGURATION SNAPSHOT</span>
          <h1>{d.title}</h1>
          <div className="metadata">
            <span className="firmware">
              {d.firmware.family} {d.firmware.version ?? "unknown version"}
              {d.firmware.boardName && ` · ${d.firmware.boardName}`}
            </span>
            {d.craftName && (
              <span className="declared-name">
                Craft <b>{d.craftName}</b>
              </span>
            )}
            {d.pilotName && (
              <span className="declared-name">
                Pilot <b>{d.pilotName}</b>
              </span>
            )}
            <span>SHA-256 {d.hash.slice(0, 12)}</span>
            <span>{d.sourceEvidence.lineCount} lines</span>
            <span className="unknown-badge">Partial · defaults unknown</span>
          </div>
        </div>
        {d.sourceId !== "virtual" && (
          <button onClick={reload}>Reload file</button>
        )}
      </section>
      {!d.sourceEvidence.hasDumpAll && (
        <section
          className="notice"
          role="alert"
          aria-label="Incomplete backup warning"
        >
          <details className="notice-details">
            <summary>
              <b>No Betaflight CLI dump all marker detected.</b>
            </summary>
            <p>
              This may be a diff or a partial backup. Missing Rates, Expo, or
              PID values cannot be assumed to be zero.
            </p>
            <p>
              For complete Rates and PID inspection, connect your configured
              flight controller to Betaflight Configurator, open the CLI, run{" "}
              <code>dump all</code>, wait for it to finish, and save or paste
              the entire output here. No reset or <code>defaults</code> command
              is needed.
            </p>
          </details>
        </section>
      )}
      <div className="tabbar" role="tablist" aria-label="Inspector views">
        {tabs.map((t) => (
          <button
            role="tab"
            aria-selected={tab === t}
            className={tab === t ? "active" : ""}
            key={t}
            onClick={() => setTab(t)}
          >
            {t}
            {t === "Audit" && (
              <span className="count">
                {inspection.audits.filter((a) => a.status === "finding").length}
              </span>
            )}
          </button>
        ))}
      </div>
      <section className="content" role="tabpanel" aria-label={tab}>
        <div className="section-heading">
          <div>
            <h2>
              {
                {
                  Rates: "Rate curves",
                  PID: "PID & feedforward",
                  Filters: "Filter configuration",
                  Ports: "Serial allocations",
                  Modes: "Mode activation",
                  OSD: "OSD layout",
                  Raw: "Source & parameters",
                  Audit: "Configuration audit",
                  Export: "Export a snippet",
                }[tab]
              }
            </h2>
            <p className="muted">
              {d.firmware.packId ?? "No compatible schema"} · Explicit source
              values
            </p>
          </div>
          <div className="profile-controls">
            {["PID", "Filters", "Export"].includes(tab) &&
              profile("PID", pidProfiles, pid, setPid)}
            {["Rates", "Export"].includes(tab) &&
              profile("Rate", rateProfiles, rate, setRate)}
          </div>
        </div>
        {!d.firmware.packId && (
          <p className="notice">
            This firmware version is not supported for Rates and PID inspection.
            FlightLens currently supports Betaflight 4.2, 4.3, 4.4, 4.5, and
            2025.12 schemas. A new backup will not resolve this compatibility
            gap. Source and parsed syntax remain inspectable; semantic views and
            export require supported firmware.
          </p>
        )}
        {tab === "Rates" && (
          <RatesInspector
            document={d}
            parameters={parameters}
            current={current}
            rateProfiles={rateProfiles}
            rate={rate}
            setRate={setRate}
            inspection={inspection}
            rateInspections={rateInspections}
            source={source}
          />
        )}
        {tab === "PID" && (
          <PIDInspector
            document={d}
            parameters={parameters}
            current={current}
            pidProfiles={pidProfiles}
            pid={pid}
            setPid={setPid}
            source={source}
          />
        )}
        {tab === "Filters" && (
          <Filters
            document={d}
            parameters={current.filter((p) =>
              /lpf|lowpass|notch|rpm_filter/.test(p.key),
            )}
            source={source}
          />
        )}
        {tab === "Ports" && <PortsInspector ports={d.ports} source={source} />}
        {tab === "Modes" && <ModesInspector modes={d.modes} source={source} />}
        {tab === "OSD" && (
          <Osd elements={inspection.osd} source={source} parameters={current} />
        )}
        {tab === "Raw" && (
          <RawInspector
            document={d}
            parameters={parameters}
            source={source}
            rawPage={rawPage}
            setRawPage={setRawPage}
            line={line}
          />
        )}
        {tab === "Audit" && (
          <AuditInspector inspection={inspection} source={source} />
        )}
        {tab === "Export" && <Export document={d} pid={pid} rate={rate} />}
      </section>
    </>
  );
}
