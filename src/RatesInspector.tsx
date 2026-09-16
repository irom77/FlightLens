import type { DocumentView, Parameter } from "./bindings/core";
import { ParameterTable } from "./ParameterTable";
import { value } from "./parameterValue";
import { MissingSettingsNotice } from "./BackupInstructions";
import type { Derived, Inspection } from "./bindings/core";
import { Plot } from "./Plots";
import { DerivedTable } from "./DerivedTable";
export function RatesInspector({
  document: d,
  parameters,
  current,
  rateProfiles,
  rate,
  setRate,
  inspection,
  rateInspections,
  source,
}: {
  document: DocumentView;
  parameters: Parameter[];
  current: Parameter[];
  rateProfiles: number[];
  rate: number;
  setRate: (n: number) => void;
  inspection: Inspection;
  rateInspections: Record<number, Inspection>;
  source: (n: number) => void;
}) {
  const profileParameters = (kind: "pid" | "rate", index: number) =>
    parameters.filter((p) => p.scope.kind === kind && p.scope.index === index);
  const profileValue = (kind: "pid" | "rate", index: number, key: string) =>
    profileParameters(kind, index).find((p) => p.key === key);
  const rateSetting = (axis: string, suffix: string) =>
    current.find(
      (p) => p.scope.kind === "rate" && p.key === `${axis}_${suffix}`,
    );
  const derivedIn = (index: number, key: string) =>
    d.derived[`rate:${index}:${key}`];
  const derivedFor = (index: number) =>
    Object.values(d.derived).filter(
      (v): v is Derived =>
        v !== undefined && v.scope.kind === "rate" && v.scope.index === index,
    );
  // A value read back from the firmware never renders like a declared one: it
  // shows the release it came from where a declared value shows a source line.
  const shown = (index: number, key: string) => {
    const p = profileValue("rate", index, key);
    if (p) return value(p);
    const v = derivedIn(index, key);
    return v ? `${v.value.value}` : null;
  };

  return (
    <>
      <div className="profile-overview">
        {rateProfiles.map((profileIndex) => {
          const values = profileParameters("rate", profileIndex);
          const result = rateInspections[profileIndex];
          const complete =
            result?.rates.filter((c) => c.points.length).length ?? 0;
          return (
            <button
              className={`profile-card ${profileIndex === rate ? "selected" : ""}`}
              key={profileIndex}
              onClick={() => setRate(profileIndex)}
            >
              <strong>Profile {profileIndex + 1}</strong>
              <span>
                {values.length} explicit settings
                {derivedFor(profileIndex).length > 0 &&
                  ` · ${derivedFor(profileIndex).length} from firmware defaults`}
              </span>
              <span>{complete}/3 rate curves available</span>
              <small>
                RC {shown(profileIndex, "roll_rc_rate") ?? "—"}
                {" · "}Super {shown(profileIndex, "roll_srate") ?? "—"}
              </small>
            </button>
          );
        })}
      </div>
      <p className="muted">
        Detailed view: Profile {rate + 1}. Select another profile above to
        switch.
      </p>
      <div className="stats">
        {inspection.rates.map((c) => {
          const input = (suffix: string) => {
            const declared = rateSetting(c.name, suffix);
            if (declared) return value(declared);
            const read = derivedIn(rate, `${c.name}_${suffix}`);
            return read ? `${read.value.value}*` : "unknown";
          };
          return (
            <div className="stat" key={c.name}>
              <span>{c.name.toUpperCase()}</span>
              <strong>
                {c.points.length
                  ? Math.round(c.points[c.points.length - 1].y)
                  : "Unavailable"}{" "}
                {c.points.length > 0 && <small>°/s</small>}
              </strong>
              <small>
                RC {input("rc_rate")} · Super {input("srate")} · Expo{" "}
                {input("expo")}
              </small>
              <small>
                {c.reason ??
                  (c.derivedInputs.length
                    ? `Full-stick static rate · ${c.derivedInputs.length} input${c.derivedInputs.length === 1 ? "" : "s"} read back from firmware defaults`
                    : "Full-stick static rate")}
              </small>
            </div>
          );
        })}
      </div>
      <div className="card">
        <div className="card-heading">
          Angular velocity <span>degrees / second</span>
        </div>
        <Plot curves={inspection.rates} />
      </div>
      <section className="card" aria-label="Throttle Curve Preview">
        <div className="card-heading">
          Throttle Curve Preview <span>throttle command (%)</span>
        </div>
        <div className="stats">
          {[
            ["thr_mid", "Throttle MID"],
            ["thr_expo", "Throttle EXPO"],
            ...(d.firmware.version === "2025.12.3-alpha.KAACK_V19"
              ? [["thr_hover", "Throttle hover"]]
              : []),
            ["throttle_limit_type", "Throttle limit"],
            ["throttle_limit_percent", "Limit percent"],
          ].map(([key, label]) => {
            const parameter = profileValue("rate", rate, key);
            return (
              <div className="stat" key={key}>
                <span>{label}</span>
                <strong>{parameter ? value(parameter) : "Unknown"}</strong>
                {parameter && (
                  <button
                    className="source-value"
                    onClick={() => source(parameter.line)}
                  >
                    Line {parameter.line}
                  </button>
                )}
                {parameter && (!parameter.valid || !parameter.supported) && (
                  <small>Invalid or unsupported input</small>
                )}
              </div>
            );
          })}
        </div>
        <Plot curves={[inspection.throttle]} throttle />
        <p className="muted">
          Configured throttle curve and limit from this rate profile. Excludes
          runtime effects such as smoothing, boost and RPM-limit bypass; this is
          not motor output or thrust.
        </p>
      </section>
      {inspection.rates.some((curve) => curve.points.length === 0) && (
        <MissingSettingsNotice />
      )}
      {d.derivedNote && (
        <p className="notice">
          <b>No firmware defaults were read back.</b> {d.derivedNote}
        </p>
      )}
      <p className="notice">
        Static rate law before downstream rate limits, smoothing, and
        camera-angle mixing. Missing inputs suppress the affected curve. A value
        marked <b>*</b> is not in this backup: the file resets the configuration
        first, so a setting it never assigns still holds the firmware default,
        listed below with the release it comes from.
      </p>
      <ParameterTable
        rows={current.filter((p) => p.scope.kind === "rate")}
        source={source}
      />
      <DerivedTable rows={derivedFor(rate)} />
    </>
  );
}
