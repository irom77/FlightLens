import type { DocumentView, Derived, Parameter } from "./bindings/core";
import { DerivedTable } from "./DerivedTable";
import { ParameterTable } from "./ParameterTable";
import { value } from "./parameterValue";
import { MissingSettingsNotice } from "./BackupInstructions";
export function PIDInspector({
  document: d,
  parameters,
  current,
  pidProfiles,
  pid,
  setPid,
  source,
}: {
  document: DocumentView;
  parameters: Parameter[];
  current: Parameter[];
  pidProfiles: number[];
  pid: number;
  setPid: (n: number) => void;
  source: (n: number) => void;
}) {
  const profileParameters = (kind: "pid" | "rate", index: number) =>
    parameters.filter((p) => p.scope.kind === kind && p.scope.index === index);
  const profileValue = (kind: "pid" | "rate", index: number, key: string) =>
    profileParameters(kind, index).find((p) => p.key === key);

  const derivedFor = (index: number) =>
    Object.values(d.derived).filter(
      (v): v is Derived =>
        v !== undefined && v.scope.kind === "pid" && v.scope.index === index,
    );
  const recovered = (index: number, key: string) =>
    d.derived[`pid:${index}:${key}`];
  const known = (index: number, key: string) => {
    const declared = profileValue("pid", index, key);
    return declared
      ? declared.valid && declared.supported
      : Boolean(recovered(index, key));
  };
  const shown = (index: number, key: string) => {
    const declared = profileValue("pid", index, key);
    return declared
      ? value(declared)
      : recovered(index, key)
        ? `${recovered(index, key)!.rawValue}*`
        : "—";
  };

  return (
    <>
      <div className="profile-overview">
        {pidProfiles.map((profileIndex) => {
          const values = profileParameters("pid", profileIndex);
          const count = ["p_roll", "i_roll", "d_roll", "f_roll"].filter((key) =>
            known(profileIndex, key),
          ).length;
          return (
            <button
              className={`profile-card ${profileIndex === pid ? "selected" : ""}`}
              key={profileIndex}
              onClick={() => setPid(profileIndex)}
            >
              <strong>Profile {profileIndex + 1}</strong>
              <span>
                {values.length} explicit settings
                {derivedFor(profileIndex).length > 0 &&
                  ` · ${derivedFor(profileIndex).length} from firmware defaults`}
              </span>
              <span>{count}/4 core PID gains available</span>
              <small>
                P {shown(profileIndex, "p_roll")}
                {" · "}I {shown(profileIndex, "i_roll")}
                {" · "}D {shown(profileIndex, "d_roll")}
              </small>
            </button>
          );
        })}
      </div>
      <p className="muted">
        Detailed view: Profile {pid + 1}. Select another profile above to
        switch.
      </p>
      <div className="card">
        <table>
          <thead>
            <tr>
              <th>Axis</th>
              {["P", "I", "D", "F", "D min", "D max"].map((k) => (
                <th key={k}>{k}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {["roll", "pitch", "yaw"].map((axis) => (
              <tr key={axis}>
                <th>{axis}</th>
                {["p", "i", "d", "f", "d_min", "d_max"].map((k) => {
                  const p = current.find(
                    (p) => p.scope.kind === "pid" && p.key === `${k}_${axis}`,
                  );
                  const derived = recovered(pid, `${k}_${axis}`);
                  return (
                    <td key={k}>
                      {p ? (
                        <button
                          className="source-value"
                          onClick={() => source(p.line)}
                        >
                          {value(p)}
                        </button>
                      ) : derived ? (
                        <span
                          title={`Betaflight ${derived.sourceVersion} default · not declared, not exported`}
                        >
                          {derived.rawValue}*
                        </span>
                      ) : (
                        <span className="muted">Unknown</span>
                      )}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {["roll", "pitch", "yaw"].some((axis) =>
        ["p", "i", "d", "f"].some((gain) => {
          return !known(pid, `${gain}_${axis}`);
        }),
      ) && <MissingSettingsNotice />}
      <DerivedTable rows={derivedFor(pid)} />
      {derivedFor(pid).length > 0 && (
        <p className="notice">
          * Read back from verified firmware defaults; not declared or exported.
          Omitted roll/pitch D and D-min/D-max remain unknown.
        </p>
      )}
      <p className="notice">
        Gains do not establish the aircraft’s physical response. Native
        D-min/D-max names are preserved; an absent name is not inferred from
        another version.
      </p>
      <ParameterTable
        rows={current.filter(
          (p) =>
            p.scope.kind === "pid" && /feedforward|d_min|d_max/.test(p.key),
        )}
        source={source}
      />
    </>
  );
}
