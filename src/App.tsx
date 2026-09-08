import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  Artifact,
  ConfigDocument,
  Derived,
  ExportRequest,
  Inspection,
  Parameter,
  Point,
  SourceDescriptor,
  ValidatedSnippet,
} from "./bindings/core";
import { api, desktopAvailable } from "./ipc/client";
import { savedActive, tabs, useWorkspace } from "./stores/workspace";
import { useTheme } from "./stores/theme";
import { Plot } from "./Plots";
import { OsdGlyphs } from "./OsdGlyphs";
import logo from "./assets/flightlens-logo.svg";
import packageJson from "../package.json";
import sample from "../fixtures/configs/betaflight-4.5.0.dump?raw";
const empty: Inspection = { rates: [], osd: [], audits: [] };
const value = (p: Parameter) => String(p.value.value);
const message = (e: unknown) =>
  typeof e === "string"
    ? e
    : e instanceof Error
      ? e.message
      : "The operation could not be completed.";

export default function App() {
  const workspace = useWorkspace();
  const theme = useTheme();
  // Parsed documents stay outside Zustand; only tab and document metadata enter the UI store.
  const repository = useRef(new Map<string, Artifact>());
  const [revision, setRevision] = useState(0);
  const [paste, setPaste] = useState(false);
  const [text, setText] = useState("");
  const [label, setLabel] = useState("Pasted config 1");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const desktop = desktopAvailable();
  const add = (artifact: Artifact) => {
    repository.current.set(artifact.document.id, artifact);
    workspace.add({
      id: artifact.document.id,
      title: artifact.document.title,
      family:
        artifact.kind === "config"
          ? artifact.document.firmware.family
          : artifact.document.family,
    });
    setRevision((n) => n + 1);
  };
  const run = async (task: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await task();
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const openSources = async (
    sources: SourceDescriptor[],
    failures: string[] = [],
  ) => {
    for (const source of sources) {
      try {
        add(await api.open(source.id));
      } catch (e) {
        failures.push(`${source.label}: ${message(e)}`);
      }
    }
    if (failures.length) setError(failures.join("\n"));
  };
  // Reopen the backups the previous run left open, then whatever was dropped on
  // the application while it was starting. The backend keeps the file list; the
  // documents themselves are parsed again from disk, so an edited backup is
  // restored as it now stands.
  const restore = async () => {
    const session = await api.restore();
    await openSources(
      session.sources,
      session.unavailable.map(
        (name) =>
          `${name}: this backup could not be reopened. It may have been moved, renamed, or deleted.`,
      ),
    );
    const previous = savedActive();
    if (
      previous &&
      useWorkspace.getState().documents.some((d) => d.id === previous)
    )
      workspace.activate(previous);
    await openSources(await api.pending());
  };
  useEffect(() => {
    let saved: string | null = null;
    try {
      saved = localStorage.getItem("flightlens.tab");
    } catch {
      /* Preference storage can be unavailable. */
    }
    if (tabs.some((t) => t === saved))
      workspace.setTab(saved as (typeof tabs)[number]);
    const shortcut = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.code === "KeyV") {
        e.preventDefault();
        setPaste(true);
      }
      if (e.key === "Escape") setPaste(false);
    };
    window.addEventListener("keydown", shortcut);
    if (!desktop) return () => window.removeEventListener("keydown", shortcut);
    let disposed = false;
    const subscriptions = [
      listen("sources-ready", () => {
        if (!disposed) void run(async () => openSources(await api.pending()));
      }),
      listen<string>("source-error", (e) => setError(e.payload)),
    ];
    void run(restore);
    return () => {
      disposed = true;
      window.removeEventListener("keydown", shortcut);
      for (const sub of subscriptions) void sub.then((unlisten) => unlisten());
    };
    // Native listeners use the stable store actions and repository ref.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const artifact = repository.current.get(workspace.activeId ?? "");
  void revision;
  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img className="brandmark" src={logo} alt="" aria-hidden="true" />
          <div>
            FlightLens
            <small>OFFLINE CONFIG INSPECTOR · v{packageJson.version}</small>
          </div>
        </div>
        <section className="notice" aria-label="Backup import warning">
          <b>For complete Rates and PID inspection: Betaflight CLI <code>dump all</code></b>
          <p>Save or paste the entire output, including the header and all profiles. Diff backups may omit Rates, Expo, and PID values.</p>
        </section>
        <button
          className="primary import-button"
          disabled={!desktop || busy}
          onClick={() =>
            void run(async () => openSources(await api.chooseFiles()))
          }
        >
          ＋ Open backups
        </button>
        <button
          className="subtle"
          disabled={!desktop || busy}
          onClick={() => setPaste(true)}
        >
          Paste configuration <kbd>⇧ ⌘ V</kbd>
        </button>
        <div className="section-label">
          OPEN DOCUMENTS <span>{workspace.documents.length}</span>
        </div>
        <nav className="documents" aria-label="Open documents">
          {workspace.documents.map((d) => (
            <div
              className={`document ${workspace.activeId === d.id ? "selected" : ""}`}
              key={d.id}
            >
              <button onClick={() => workspace.activate(d.id)}>
                <span className="file-icon">≡</span>
                <span>
                  {d.title}
                  <small>{d.family}</small>
                </span>
              </button>
              <button
                aria-label={`Close ${d.title}`}
                onClick={() =>
                  void run(async () => {
                    await api.close(d.id);
                    repository.current.delete(d.id);
                    workspace.close(d.id);
                  })
                }
              >
                ×
              </button>
            </div>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <span className="status-dot" /> Local by design
          <p>
            Backups stay on this device.
            <br />
            Original files are never changed.
          </p>
          <small>PHASE 1 · DEVELOPMENT BUILD · v{packageJson.version}</small>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            Workspace <span className="slash">/</span>{" "}
            {artifact?.document.title ?? "Overview"}
          </span>
          <div className="topbar-actions">
            <button
              className="theme-toggle"
              onClick={theme.toggle}
              aria-label={`${theme.theme === "dark" ? "Dark" : "Light"} mode. Switch to ${theme.theme === "dark" ? "light" : "dark"} mode.`}
            >
              <span aria-hidden="true">
                {theme.theme === "dark" ? "☾" : "☀"}
              </span>
              {theme.theme === "dark" ? "Dark" : "Light"} mode
            </button>
            <span className="pill">● Offline</span>
          </div>
        </header>
        {error && (
          <div className="error" role="alert">
            {error}
            <button aria-label="Dismiss error" onClick={() => setError("")}>
              ×
            </button>
          </div>
        )}
        {busy && (
          <div className="working" role="status">
            Processing locally…
          </div>
        )}
        {!artifact ? (
          <div className="welcome">
            <span className="eyebrow">YOUR BACKUPS, MADE READABLE</span>
            <h1>
              A clearer view
              <br />
              of your flight controller.
            </h1>
            <p>
              Inspect rates, trace settings, and review your setup.
              <br />
              All from a backup. All offline.
            </p>
            <div className="dropzone">
              <span className="drop-icon">⇣</span>
              <h2>Drop a flight controller backup</h2>
              <p>.txt · .diff · .dump · .bbl · .bfl · .param · .parm</p>
              <div className="actions">
                <button
                  className="primary"
                  disabled={!desktop || busy}
                  onClick={() =>
                    void run(async () => openSources(await api.chooseFiles()))
                  }
                >
                  Browse files
                </button>
                <button
                  disabled={!desktop || busy}
                  onClick={() => setPaste(true)}
                >
                  Paste CLI text
                </button>
              </div>
            </div>
            <BackupInstructions />
            <button
              className="text-button"
              disabled={!desktop || busy}
              onClick={() =>
                void run(async () =>
                  add(await api.ingest(sample, "Example · Betaflight 4.5.0")),
                )
              }
            >
              Explore a synthetic example →
            </button>
            {!desktop && (
              <p className="notice">
                This is the web preview. Launch with <code>pnpm tauri dev</code>{" "}
                to import and analyze backups using the local Rust core.
              </p>
            )}
            <div className="welcome-notes">
              <span>
                01 <b>Read-only snapshots</b>
              </span>
              <span>
                02 <b>Source-linked values</b>
              </span>
              <span>
                03 <b>No guessed defaults</b>
              </span>
            </div>
          </div>
        ) : artifact.kind === "recognized" ? (
          <section className="content">
            <span className="eyebrow">RECOGNIZED ARTIFACT</span>
            <h1>{artifact.document.family}</h1>
            <p>{artifact.document.message}</p>
          </section>
        ) : (
          <Inspector
            key={artifact.document.id}
            document={artifact.document}
            onError={setError}
            reload={() =>
              void run(async () =>
                add(await api.open(artifact.document.sourceId)),
              )
            }
          />
        )}
      </main>
      {paste && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="paste-title"
          >
            <div className="section-heading">
              <h2 id="paste-title">Paste a configuration</h2>
              <button
                aria-label="Close paste dialog"
                onClick={() => setPaste(false)}
              >
                ×
              </button>
            </div>
            <p>
              Paste CLI backup text, including its firmware header and profile
              selectors. Maximum 16 MiB.
            </p>
            <BackupInstructions />
            <label>
              Document name
              <input
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                maxLength={200}
              />
            </label>
            <label>
              Backup text
              <textarea
                autoFocus
                value={text}
                onChange={(e) => setText(e.target.value)}
                spellCheck={false}
                placeholder="# Betaflight / …"
              />
            </label>
            <div className="actions">
              <button onClick={() => setPaste(false)}>Cancel</button>
              <button
                className="primary"
                disabled={busy || !text.trim() || !desktop}
                onClick={() =>
                  void run(async () => {
                    add(
                      await api.ingest(
                        text,
                        label.trim() || "Pasted configuration",
                      ),
                    );
                    setText("");
                    setPaste(false);
                  })
                }
              >
                Inspect configuration
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

export function defaultProfile(profiles: number[]): number {
  return profiles.includes(0) ? 0 : (profiles[0] ?? 0);
}

export function populatedProfiles(
  d: ConfigDocument,
  kind: "pid" | "rate",
): number[] {
  const settings = [
    ...Object.values(d.parameters),
    ...Object.values(d.derived),
  ];
  return (kind === "pid" ? d.pidProfiles : d.rateProfiles).filter((index) =>
    settings.some(
      (setting) =>
        setting?.scope.kind === kind && setting.scope.index === index,
    ),
  );
}

function BackupInstructions() {
  return (
    <section className="notice" aria-label="Expected backup format">
      <b>For complete Rates and PID inspection: Betaflight CLI dump all</b>
      <ol>
        <li>
          Connect the configured flight controller to Betaflight Configurator
          and open the CLI tab.
        </li>
        <li>
          Type <code>dump all</code> and press Enter. Wait for the output to
          finish.
        </li>
        <li>
          Save the entire output as a text file, including the firmware header
          and all profiles, then open it here. You can also paste the entire
          output.
        </li>
      </ol>
      <p>
        This includes unchanged settings. A <code>diff</code> or{" "}
        <code>diff all</code> backup is accepted, but omitted Rates, Expo, or
        PID values may remain unknown. No reset or <code>defaults</code> command
        is needed. FlightLens currently supports Betaflight 4.2, 4.3, 4.4, and
        4.5 schemas; a full dump does not add support for other firmware
        versions.
      </p>
    </section>
  );
}

function MissingSettingsNotice() {
  return (
    <>
      <p className="notice" role="status">
        <b>Incomplete inspection data.</b> Unknown means this backup does not
        establish the value; it does not mean zero. For complete inspection,
        capture
        <code> dump all</code> as described below. Omitted vendor defaults
        cannot be reconstructed reliably from a diff.
      </p>
      <BackupInstructions />
    </>
  );
}

function Inspector({
  document: d,
  onError,
  reload,
}: {
  document: ConfigDocument;
  onError: (e: string) => void;
  reload: () => void;
}) {
  const { tab, setTab } = useWorkspace();
  // Keep original CLI numbers, but skip sections with no inspection data.
  const pidProfiles = populatedProfiles(d, "pid");
  const rateProfiles = populatedProfiles(d, "rate");
  const [pid, setPid] = useState(() => defaultProfile(pidProfiles));
  const [rate, setRate] = useState(() => defaultProfile(rateProfiles));
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
  useEffect(() => {
    if (tab === "Raw" && line)
      document
        .getElementById(`line-${line}`)
        ?.scrollIntoView({ block: "center" });
  }, [tab, line, rawPage]);
  const parameters = Object.values(d.parameters).filter(
    (p): p is Parameter => p !== undefined,
  );
  const current = parameters.filter(
    (p) =>
      p.scope.kind === "global" ||
      (p.scope.kind === "pid" && p.scope.index === pid) ||
      (p.scope.kind === "rate" && p.scope.index === rate),
  );
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
            <span>{d.syntax.length} lines</span>
            <span className="unknown-badge">Partial · defaults unknown</span>
          </div>
        </div>
        {d.sourceId !== "virtual" && (
          <button onClick={reload}>Reload file</button>
        )}
      </section>
      {!d.syntax.some((line) => /^\s*(?:#\s*)?dump\s+all\s*$/i.test(line.raw)) && (
        <section className="notice" role="alert" aria-label="Incomplete backup warning">
          <b>No Betaflight CLI dump all marker detected.</b>
          <p>This may be a diff or a partial backup. Missing Rates, Expo, or PID values cannot be assumed to be zero.</p>
          <p>For complete Rates and PID inspection, connect your configured flight controller to Betaflight Configurator, open the CLI, run <code>dump all</code>, wait for it to finish, and save or paste the entire output here. No reset or <code>defaults</code> command is needed.</p>
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
            FlightLens currently supports Betaflight 4.2, 4.3, 4.4, and 4.5
            schemas. A new backup will not resolve this compatibility gap.
            Source and parsed syntax remain inspectable; semantic views and
            export require supported firmware.
          </p>
        )}
        {tab === "Rates" && (
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
              camera-angle mixing. Missing inputs suppress the affected curve. A
              value marked <b>*</b> is not in this backup: the file resets the
              configuration first, so a setting it never assigns still holds the
              firmware default, listed below with the release it comes from.
            </p>
            <ParameterTable
              rows={current.filter((p) => p.scope.kind === "rate")}
              source={source}
            />
            <DerivedTable rows={derivedFor(rate)} />
          </>
        )}
        {tab === "PID" && (
          <>
            <div className="profile-overview">
              {pidProfiles.map((profileIndex) => {
                const values = profileParameters("pid", profileIndex);
                const known = ["p_roll", "i_roll", "d_roll", "f_roll"].filter(
                  (key) => profileValue("pid", profileIndex, key)?.valid,
                ).length;
                return (
                  <button
                    className={`profile-card ${profileIndex === pid ? "selected" : ""}`}
                    key={profileIndex}
                    onClick={() => setPid(profileIndex)}
                  >
                    <strong>Profile {profileIndex + 1}</strong>
                    <span>{values.length} explicit settings</span>
                    <span>{known}/4 core PID gains available</span>
                    <small>
                      P{" "}
                      {profileValue("pid", profileIndex, "p_roll")
                        ? value(profileValue("pid", profileIndex, "p_roll")!)
                        : "—"}
                      {" · "}I{" "}
                      {profileValue("pid", profileIndex, "i_roll")
                        ? value(profileValue("pid", profileIndex, "i_roll")!)
                        : "—"}
                      {" · "}D{" "}
                      {profileValue("pid", profileIndex, "d_roll")
                        ? value(profileValue("pid", profileIndex, "d_roll")!)
                        : "—"}
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
                          (p) =>
                            p.scope.kind === "pid" && p.key === `${k}_${axis}`,
                        );
                        return (
                          <td key={k}>
                            {p ? (
                              <button
                                className="source-value"
                                onClick={() => source(p.line)}
                              >
                                {value(p)}
                              </button>
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
                const parameter = profileValue("pid", pid, `${gain}_${axis}`);
                return !parameter || !parameter.valid || !parameter.supported;
              }),
            ) && <MissingSettingsNotice />}
            <p className="notice">
              Gains do not establish the aircraft’s physical response. Native
              D-min/D-max names are preserved; an absent name is not inferred
              from another version.
            </p>
            <ParameterTable
              rows={current.filter(
                (p) =>
                  p.scope.kind === "pid" &&
                  /feedforward|d_min|d_max/.test(p.key),
              )}
              source={source}
            />
          </>
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
        {tab === "Ports" && (
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
                  {d.ports.map((p) => (
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
              {!d.ports.length && (
                <Empty>
                  No serial allocations are present in this artifact.
                </Empty>
              )}
            </div>
            <p className="notice">
              Multiple function bits can represent allowed sharing. Hardware
              availability and build-specific sharing validation remain unknown.
            </p>
          </>
        )}
        {tab === "Modes" && (
          <>
            <div className="card">
              {d.modes.map((m) => (
                <div className="mode-row" key={m.index}>
                  <div>
                    <button
                      className="source-value"
                      onClick={() => source(m.line)}
                    >
                      {m.name}
                    </button>
                    <small>
                      Slot {m.index} ·{" "}
                      {m.channelAssigned
                        ? `AUX ${m.channel + 1}`
                        : "no channel assigned"}{" "}
                      ·{" "}
                      {m.logic === null
                        ? "logic unspecified"
                        : m.logic === 0
                          ? "OR"
                          : "AND"}
                      {m.linked ? ` · linked ID ${m.linked}` : ""}
                    </small>
                  </div>
                  <div className="range-track">
                    <span
                      style={{
                        left: `${(m.start - 900) / 12}%`,
                        width: `${(m.end - m.start) / 12}%`,
                      }}
                    />
                    <small>
                      {m.start} — {m.end} μs
                      {m.start === m.end ? " · inactive" : ""}
                    </small>
                  </div>
                </div>
              ))}
              {!d.modes.length && (
                <Empty>
                  No explicit mode ranges. Omitted ranges remain unknown.
                </Empty>
              )}
            </div>
            <p className="notice">
              AUX numbering is displayed from 1; CLI channels are zero-based.
              Inactive ranges and linked-mode relationships are preserved. A
              mode with no channel assigned is shown as written; Betaflight
              discards such a row if it is pasted back.
            </p>
          </>
        )}
        {tab === "OSD" && (
          <Osd elements={inspection.osd} source={source} parameters={current} />
        )}
        {tab === "Raw" && (
          <>
            <details>
              <summary>
                Normalized parameter table · {parameters.length} fields
              </summary>
              <ParameterTable rows={parameters} source={source} />
            </details>
            <details>
              <summary>Parser diagnostics · {d.diagnostics.length}</summary>
              {d.diagnostics.map((diag, i) => (
                <p className="diagnostic" key={i}>
                  <b>{diag.severity}</b>{" "}
                  {diag.line && (
                    <button
                      className="source-value"
                      onClick={() => source(diag.line!)}
                    >
                      Line {diag.line}
                    </button>
                  )}{" "}
                  {diag.message}
                </p>
              ))}
            </details>
            <div className="actions">
              <button
                disabled={rawPage === 0}
                onClick={() => setRawPage((p) => p - 1)}
              >
                Previous lines
              </button>
              <span className="muted">
                Lines {rawPage * 500 + 1}–
                {Math.min((rawPage + 1) * 500, d.syntax.length)} of{" "}
                {d.syntax.length}
              </span>
              <button
                disabled={(rawPage + 1) * 500 >= d.syntax.length}
                onClick={() => setRawPage((p) => p + 1)}
              >
                Next lines
              </button>
            </div>
            <div className="raw-source">
              {d.syntax.slice(rawPage * 500, (rawPage + 1) * 500).map((l) => (
                <div
                  id={`line-${l.line}`}
                  className={line === l.line ? "highlight-line" : ""}
                  key={l.line}
                >
                  <span>{l.line}</span>
                  <code>{l.raw.replace(/\r?\n$/, "") || " "}</code>
                  <small>
                    {["unsupported", "malformed"].includes(l.command.kind)
                      ? l.command.kind
                      : ""}
                  </small>
                </div>
              ))}
            </div>
          </>
        )}
        {tab === "Audit" && (
          <>
            <p className="notice">
              {
                inspection.audits.filter((a) =>
                  ["pass", "finding"].includes(a.status),
                ).length
              }{" "}
              / {inspection.audits.length} rules evaluated. No findings does not
              mean safe to fly.
            </p>
            {inspection.audits.map((a) => (
              <article className="audit-card" key={a.id}>
                <div>
                  <span className={`audit-status ${a.status}`}>
                    {a.status.replaceAll("_", " ")}
                  </span>
                  <small>{a.severity}</small>
                </div>
                <h3>{a.id.replaceAll("-", " ")}</h3>
                <p>{a.explanation}</p>
                <div className="actions">
                  {a.lines.map((n) => (
                    <button
                      className="source-value"
                      key={n}
                      onClick={() => source(n)}
                    >
                      Line {n}
                    </button>
                  ))}
                  <small className="muted" title={a.reference}>
                    Reference: Betaflight {d.firmware.version} ·{" "}
                    {a.reference.split("/src/main/")[1]}
                  </small>
                </div>
              </article>
            ))}
          </>
        )}
        {tab === "Export" && <Export document={d} pid={pid} rate={rate} />}
      </section>
    </>
  );
}
function Empty({ children }: { children: React.ReactNode }) {
  return <div className="empty">{children}</div>;
}
/// Values no source line declares. Kept out of the parameter table on purpose:
/// these have no line to jump to, and must never read as though they do.
function DerivedTable({ rows }: { rows: Derived[] }) {
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
function ParameterTable({
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
function Filters({
  document: d,
  parameters,
  source,
}: {
  document: ConfigDocument;
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
  useEffect(() => {
    setPoints([]);
    setError("");
  }, [sampleRate, selected, parameters.map((p) => p.line).join(",")]);
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
function Osd({
  elements,
  source,
  parameters,
}: {
  elements: Inspection["osd"];
  source: (n: number) => void;
  parameters: Parameter[];
}) {
  const [display, setDisplay] = useState("auto");
  const [profile, setProfile] = useState(0);
  const num = (key: string) => {
    const p = parameters.find((p) => p.key === key && p.valid);
    return p && typeof p.value.value === "number" ? p.value.value : undefined;
  };
  const knownVideo = parameters.find(
    (p) => p.key === "vcd_video_system" && p.valid,
  )?.value.value;
  const mode =
    display === "auto" ? String(knownVideo ?? "PAL").toUpperCase() : display;
  const savedWidth = num("osd_canvas_width");
  const savedHeight = num("osd_canvas_height");
  const cols =
    savedWidth && savedWidth > 0 ? savedWidth : mode === "HD" ? 53 : 30;
  const rows =
    savedHeight && savedHeight > 0
      ? savedHeight
      : mode === "HD"
        ? 20
        : mode === "NTSC"
          ? 13
          : 16;
  return (
    <>
      <div className="actions">
        <label>
          Display assumption
          <select value={display} onChange={(e) => setDisplay(e.target.value)}>
            <option value="auto">From source, otherwise PAL assumption</option>
            <option value="PAL">PAL · 30 × 16</option>
            <option value="NTSC">NTSC · 30 × 13</option>
            <option value="HD">HD · 53 × 20</option>
          </select>
        </label>
        <label>
          Visibility profile
          <select
            value={profile}
            onChange={(e) => setProfile(Number(e.target.value))}
          >
            {[0, 1, 2].map((n) => (
              <option key={n} value={n}>
                {n + 1}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div
        className="osd-canvas"
        style={{
          aspectRatio: `${Math.max(1, cols)} / ${Math.max(1, rows)}`,
          backgroundSize: `${100 / Math.max(1, cols)}% ${100 / Math.max(1, rows)}%`,
        }}
      >
        <span className="canvas-label">
          {cols} × {rows} · POSITION PREVIEW
        </span>
        {elements
          .filter(
            (e) =>
              (e.visibleProfiles & (1 << profile)) !== 0 &&
              e.x < cols &&
              e.y < rows,
          )
          .map((e) => (
            <button
              key={e.key}
              className="osd-marker"
              title={`${e.key} · (${e.x}, ${e.y}) · type ${e.displayType}`}
              style={{
                left: `${(e.x / cols) * 100}%`,
                top: `${(e.y / rows) * 100}%`,
                width: `${(Array.from(e.preview).length / cols) * 100}%`,
                height: `${100 / rows}%`,
              }}
              onClick={() => source(e.line)}
              aria-label={e.key}
            >
              <OsdGlyphs text={e.preview} />
            </button>
          ))}
      </div>
      <p className="notice">
        Bundled Betaflight default font. Numeric values and glyph footprints are
        illustrative samples, not telemetry. Unsupported elements use a +
        marker. Saved canvas dimensions take precedence when present.
      </p>
      <div className="card">
        <table>
          <thead>
            <tr>
              <th>Element</th>
              <th>Position</th>
              <th>Profiles</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {elements.map((e) => (
              <tr key={e.key}>
                <td>
                  <button
                    className="source-value"
                    onClick={() => source(e.line)}
                  >
                    {e.key}
                  </button>
                </td>
                <td>
                  {e.x}, {e.y}
                </td>
                <td>
                  {[0, 1, 2]
                    .filter((n) => e.visibleProfiles & (1 << n))
                    .map((n) => n + 1)
                    .join(", ") || "Hidden"}
                </td>
                <td>
                  {e.x + Array.from(e.preview).length > cols || e.y >= rows
                    ? "Sample footprint out of bounds"
                    : e.preview === "+"
                      ? "Footprint unknown"
                      : "Sample in bounds"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {!elements.length && (
          <Empty>No supported explicit OSD positions.</Empty>
        )}
      </div>
    </>
  );
}
function Export({
  document: d,
  pid,
  rate,
}: {
  document: ConfigDocument;
  pid: number;
  rate: number;
}) {
  const [groups, setGroups] = useState<string[]>(["rates"]);
  const [includeSave, setIncludeSave] = useState(false);
  const [validated, setValidated] = useState<{
    key: string;
    snippet: ValidatedSnippet;
  } | null>(null);
  const [status, setStatus] = useState("");
  const request: ExportRequest = {
    groups,
    pidProfile: pid,
    rateProfile: rate,
    destinationHeader: d.firmware.header ?? "",
    includeSave,
  };
  const requestKey = JSON.stringify(request);
  const snippet = validated?.key === requestKey ? validated.snippet : null;
  useEffect(() => {
    setValidated(null);
    setStatus("");
  }, [groups, pid, rate, includeSave]);
  return (
    <>
      <p>
        Export explicit settings from this donor. Firmware and target must match
        the original header; cross-target export is not certified.
      </p>
      <div className="card export-options">
        <small>DESTINATION · SOURCE IDENTITY</small>
        <code>{d.firmware.header ?? "Unknown — export unavailable"}</code>
        <div className="actions">
          {[
            ["rates", "Rates"],
            ["pids_filters", "PID / Filters"],
            ["modes", "Modes"],
            ["osd", "OSD"],
            ["serial", "Serial"],
            ["vtx", "VTX"],
          ].map(([key, label]) => (
            <label className="checkbox" key={key}>
              <input
                type="checkbox"
                checked={groups.includes(key)}
                onChange={(e) =>
                  setGroups(
                    e.target.checked
                      ? [...groups, key]
                      : groups.filter((g) => g !== key),
                  )
                }
              />
              {label}
            </label>
          ))}
        </div>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={includeSave}
            onChange={(e) => setIncludeSave(e.target.checked)}
          />
          Include device “save” command
        </label>
        <button
          className="primary"
          disabled={!d.firmware.packId || !groups.length}
          onClick={async () => {
            setStatus("");
            setValidated(null);
            try {
              setValidated({
                key: requestKey,
                snippet: await api.export(d.id, request),
              });
            } catch (e) {
              setStatus(message(e));
            }
          }}
        >
          Validate & preview
        </button>
      </div>
      {status && (
        <p role="status" className="notice">
          {status}
        </p>
      )}
      {snippet && (
        <>
          {snippet.additions.map((a) => (
            <p className="muted" key={a}>
              Dependency: {a}
            </p>
          ))}
          {snippet.warnings.map((w) => (
            <p className="notice" key={w}>
              {w}
            </p>
          ))}
          <pre className="snippet">{snippet.text}</pre>
          <div className="actions">
            <button
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(snippet.text);
                  setStatus("Copied validated snippet.");
                } catch {
                  setStatus(
                    "Clipboard unavailable. Use Save As or select the preview text.",
                  );
                }
              }}
            >
              Copy snippet
            </button>
            <button
              onClick={async () => {
                try {
                  setStatus(
                    (await api.save(d.id, request))
                      ? "Saved snippet to a new file."
                      : "Save cancelled.",
                  );
                } catch (e) {
                  setStatus(message(e));
                }
              }}
            >
              Save As…
            </button>
          </div>
        </>
      )}
    </>
  );
}
