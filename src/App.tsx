import { Inspector } from "./Inspector";
import { BackupInstructions } from "./BackupInstructions";
import { message } from "./errorMessage";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { WorkspacePage, SourceDescriptor } from "./bindings/core";
import { api, desktopAvailable } from "./ipc/client";
import { savedActive, tabs, useWorkspace } from "./stores/workspace";
import { useTheme } from "./stores/theme";
import { WorkspaceExplorer } from "./WorkspaceExplorer";
import {
  defaultProfiles,
  restoredProfiles,
  useSelections,
} from "./stores/selections";
import { Comparison } from "./Comparison";
import { Feedback } from "./Feedback";
import { Modal } from "./Modal";
import logo from "./assets/flightlens-logo.svg";
import packageJson from "../package.json";
import sample from "../fixtures/configs/betaflight-4.5.0.dump?raw";
export default function App() {
  const workspace = useWorkspace();
  const theme = useTheme();
  const [comparing, setComparing] = useState(false);
  const [paste, setPaste] = useState(false);
  const [feedback, setFeedback] = useState(false);
  const [text, setText] = useState("");
  const [label, setLabel] = useState("Pasted config 1");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [sessionWorkspace, setSessionWorkspace] =
    useState<WorkspacePage | null>(null);
  const [canRetrySession, setCanRetrySession] = useState(false);
  const [sessionStatus, setSessionStatus] = useState("");
  const [preserveUnavailableSelections, setPreserveUnavailableSelections] =
    useState(true);
  const desktop = desktopAvailable();
  const add = workspace.add;
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
        if (!document.querySelector('[aria-modal="true"]')) setPaste(true);
      }
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
    // Native listeners use the stable store actions.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const artifact = workspace.artifacts.get(workspace.activeId ?? "");
  const openSession = (retry: boolean, relink = false) =>
    run(async () => {
      setSessionStatus("");
      const result = await api.openPortableSession(retry, relink);
      if (!result) return;
      setCanRetrySession(true);
      setPreserveUnavailableSelections(true);
      if (result.workspace) setSessionWorkspace(result.workspace);
      for (const artifact of result.artifacts) add(artifact);
      if (result.activeId) workspace.activate(result.activeId);
      if (tabs.some((tab) => tab === result.tab))
        workspace.setTab(result.tab as (typeof tabs)[number]);
      if (result.theme === "dark" || result.theme === "light")
        theme.setTheme(result.theme);
      const warnings = [...result.warnings];
      const profiles = { ...useSelections.getState().profiles };
      for (const entry of result.profiles ?? []) {
        const artifact = useWorkspace
          .getState()
          .artifacts.get(entry.documentId);
        if (artifact?.kind !== "config") continue;
        const restored = restoredProfiles(artifact.document, {
          rate: entry.rateProfile,
          pid: entry.pidProfile,
        });
        profiles[entry.documentId] = restored.profiles;
        warnings.push(...restored.warnings);
      }
      useSelections.setState({
        profiles,
        comparison: result.comparison ?? null,
      });
      setComparing(Boolean(result.comparison));
      setSessionStatus(
        [
          `Opened ${result.artifacts.length} session backups.`,
          ...warnings,
        ].join("\n"),
      );
    });
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
          <details className="notice-details">
            <summary>
              <b>
                For complete Rates and PID inspection: Betaflight CLI{" "}
                <code>dump all</code>
              </b>
            </summary>
            <p>
              Save or paste the entire output, including the header and all
              profiles. Diff backups may omit Rates, Expo, and PID values.
            </p>
          </details>
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
        <button
          onClick={() => setComparing(!comparing)}
          aria-pressed={comparing}
        >
          {comparing ? "Back to inspector" : "Compare backups"}
        </button>
        <WorkspaceExplorer
          restoredPage={sessionWorkspace}
          disabled={!desktop || busy}
          onOpen={(artifact) => {
            add(artifact);
            setComparing(false);
          }}
        />
        <div className="section-label">
          OPEN DOCUMENTS <span>{workspace.documents.length}</span>
        </div>
        <nav className="documents" aria-label="Open documents">
          {workspace.documents.map((d) => (
            <div
              className={`document ${workspace.activeId === d.id ? "selected" : ""}`}
              key={d.id}
            >
              <button
                onClick={() => {
                  setComparing(false);
                  workspace.activate(d.id);
                }}
              >
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
            <button
              disabled={!desktop || busy}
              onClick={() => setFeedback(true)}
            >
              Send feedback
            </button>
            <span className="pill">● Offline</span>
          </div>
        </header>
        <section className="session-controls" aria-label="Portable sessions">
          <div className="session-actions">
            <button
              disabled={!desktop || busy}
              onClick={() => void openSession(false)}
            >
              Open session…
            </button>
            <button
              disabled={!desktop || busy}
              onClick={() =>
                void run(async () => {
                  setSessionStatus("");
                  const selections = useSelections.getState();
                  const documentIds = workspace.documents.map((d) => d.id);
                  const comparison = comparing
                    ? (selections.comparison ?? {
                        documents: Array.from(workspace.artifacts.values())
                          .flatMap((a) =>
                            a.kind === "config" ? [a.document.id] : [],
                          )
                          .slice(0, 2),
                        baseline: null,
                      })
                    : null;
                  if (
                    comparison &&
                    (comparison.documents.filter(Boolean).length < 2 ||
                      comparison.documents
                        .filter(Boolean)
                        .some((id) => !documentIds.includes(id)) ||
                      !comparison.documents[0] ||
                      !comparison.documents[1])
                  ) {
                    throw new Error(
                      "Select two or three available comparison backups before saving, or return to the inspector.",
                    );
                  }
                  const saved = await api.savePortableSession({
                    preserveUnavailableSelections,
                    documentIds,
                    activeId: workspace.activeId,
                    tab: workspace.tab,
                    theme: theme.theme,
                    profiles: documentIds.map((documentId) => {
                      const artifact = workspace.artifacts.get(documentId);
                      const selected =
                        selections.profiles[documentId] ??
                        (artifact?.kind === "config"
                          ? defaultProfiles(artifact.document)
                          : { rate: 0, pid: 0 });
                      return {
                        documentId,
                        rateProfile: selected.rate,
                        pidProfile: selected.pid,
                      };
                    }),
                    comparison: comparison
                      ? {
                          ...comparison,
                          documents: comparison.documents.filter(Boolean),
                        }
                      : null,
                  });
                  setSessionStatus(
                    saved
                      ? "Saved session references to a new file."
                      : "Session save cancelled.",
                  );
                })
              }
            >
              Save session as…
            </button>
          </div>
          <details>
            <summary>Session options</summary>
            <p>
              Save your open backups, profiles, comparison and workspace
              location for later. Sessions reference your backup files; they do
              not copy them. Close pasted backups and Blackbox files before
              saving.
            </p>
            <button
              disabled={!desktop || busy || !canRetrySession}
              onClick={() => void openSession(true)}
            >
              Retry session
            </button>
            <button
              disabled={!desktop || busy || !canRetrySession}
              onClick={() => void openSession(true, true)}
            >
              Relink session backups…
            </button>
            <small>
              Relink retries the saved session and asks for identical
              replacements for unavailable backups. Save As keeps the new
              locations.
            </small>
            <p>
              Retry rereads the last confirmed session and restores its saved
              selections.
            </p>
            <label>
              <input
                type="checkbox"
                disabled={busy}
                checked={preserveUnavailableSelections}
                onChange={(event) =>
                  setPreserveUnavailableSelections(event.target.checked)
                }
              />{" "}
              Keep unavailable active/comparison selections when saving
            </label>
          </details>
        </section>
        {sessionStatus && (
          <p
            className="notice"
            role="status"
            style={{ whiteSpace: "pre-wrap" }}
          >
            {sessionStatus}
          </p>
        )}
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
        {comparing ? (
          <Comparison
            documents={Array.from(workspace.artifacts.values()).flatMap((a) =>
              a.kind === "config" ? [a.document] : [],
            )}
          />
        ) : !artifact ? (
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
      {feedback && (
        <Feedback
          configId={artifact?.kind === "config" ? artifact.document.id : null}
          documentTitle={artifact?.document.title ?? null}
          onClose={() => setFeedback(false)}
        />
      )}
      {paste && (
        <Modal labelledBy="paste-title" onClose={() => setPaste(false)}>
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
              data-modal-initial-focus
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
        </Modal>
      )}
    </div>
  );
}
