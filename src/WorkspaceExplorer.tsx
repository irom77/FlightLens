import { useEffect, useRef, useState } from "react";
import { api } from "./ipc/client";
import type { ArtifactView, WorkspacePage } from "./bindings/core";

export function WorkspaceExplorer({
  disabled,
  restoredPage,
  onOpen,
}: {
  disabled: boolean;
  restoredPage?: WorkspacePage | null;
  onOpen: (artifact: ArtifactView) => void;
}) {
  const [page, setPage] = useState<WorkspacePage | null>(null);
  const [query, setQuery] = useState("");
  const [offset, setOffset] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request = useRef({ query, offset });
  if (request.current.query !== query || request.current.offset !== offset)
    request.current = { query, offset };
  const paused = useRef(disabled);
  paused.current = disabled;
  const active = useRef(true);
  const operation = useRef(false);
  const action = useRef(false);
  const last = useRef<{ query: string; offset: number } | null>(null);
  const scanning = useRef(true);
  const [running, setRunning] = useState(false);
  useEffect(() => {
    if (!restoredPage) return;
    // Replace the request identity so an old in-flight page cannot overwrite
    // the restored folder, even when its search and offset were also empty.
    request.current = { query: "", offset: 0 };
    last.current = null;
    setPage(restoredPage);
    setQuery("");
    setOffset(0);
    setError("");
    scanning.current = restoredPage.status === "scanning";
    setRunning(true);
  }, [restoredPage]);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
      void api.cancelWorkspace().catch(() => {});
    };
  }, []);
  useEffect(() => {
    if (!running) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      if (
        paused.current ||
        operation.current ||
        action.current ||
        (!scanning.current && last.current === request.current)
      ) {
        timer = setTimeout(poll, 150);
        return;
      }
      operation.current = true;
      const selected = request.current;
      try {
        const next = await api.workspacePage(selected.query, selected.offset);
        if (!disposed) {
          last.current = selected;
          scanning.current = next?.status === "scanning";
          if (selected === request.current) setPage(next);
        }
      } catch (e) {
        if (!disposed) {
          setError(String(e));
          setRunning(false);
        }
      } finally {
        operation.current = false;
      }
      if (!disposed) timer = setTimeout(poll, 250);
    };
    timer = setTimeout(poll, 0);
    return () => {
      disposed = true;
      clearTimeout(timer);
    };
  }, [running]);
  const choose = async (refresh: boolean) => {
    if (action.current) return;
    action.current = true;
    setBusy(true);
    setError("");
    while (operation.current)
      await new Promise((resolve) => setTimeout(resolve, 25));
    if (!active.current) {
      action.current = false;
      return;
    }
    operation.current = true;
    try {
      const next = await api.chooseWorkspace(refresh);
      if (next && active.current) {
        setPage(next);
        setQuery("");
        setOffset(0);
        last.current = null;
        scanning.current = true;
        setRunning(true);
      }
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      operation.current = false;
      action.current = false;
      if (active.current) setBusy(false);
    }
  };
  return (
    <section aria-label="Workspace explorer" className="workspace-explorer">
      <button disabled={disabled || busy} onClick={() => void choose(false)}>
        Choose workspace folder…
      </button>
      {page && (
        <>
          <p className="muted" title={page.root}>
            {page.root}
          </p>
          <div className="actions">
            <button
              disabled={disabled || busy}
              onClick={() => void choose(true)}
            >
              Refresh workspace
            </button>
            <button
              disabled={page.status !== "scanning"}
              onClick={() =>
                void api.cancelWorkspace().catch((e) => setError(String(e)))
              }
            >
              Cancel scan
            </button>
          </div>
          <p role="status">
            {page.status} · {page.indexed} indexed · {page.visited} entries
            checked · {page.skipped} skipped
          </p>
          {page.status === "limited" && (
            <p className="notice">
              Scan limit reached. Choose a smaller folder to find more backups.
            </p>
          )}
          {page.status === "unavailable" && (
            <p className="notice">
              Folder disconnected or unavailable. Reconnect it and refresh.
            </p>
          )}
          <input
            aria-label="Search workspace"
            placeholder="Search file names and folders…"
            maxLength={512}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setOffset(0);
            }}
          />
          <nav aria-label="Indexed backups">
            {page.entries.map((entry) => (
              <button
                key={entry.id}
                disabled={disabled || busy}
                onClick={async () => {
                  setBusy(true);
                  setError("");
                  try {
                    onOpen(await api.openWorkspaceEntry(entry.id));
                  } catch (e) {
                    setError(String(e));
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                {entry.relativePath}
                <small>
                  {Math.ceil(entry.bytes / 1024)} KiB ·{" "}
                  {entry.modifiedSeconds === null
                    ? "Date unknown"
                    : new Date(entry.modifiedSeconds * 1000).toLocaleString()}
                </small>
              </button>
            ))}
          </nav>
          <div className="actions">
            <button
              disabled={page.offset === 0}
              onClick={() => setOffset(Math.max(0, page.offset - 50))}
            >
              Previous files
            </button>
            <span>
              {page.total ? page.offset + 1 : 0}–
              {page.offset + page.entries.length} of {page.total}
            </span>
            <button
              disabled={page.offset + 50 >= page.total}
              onClick={() => setOffset(page.offset + 50)}
            >
              Next files
            </button>
          </div>
          <p className="muted">
            Metadata only; files are read when opened. Refresh after files
            change or a drive reconnects. Skips links, inaccessible entries and
            folders deeper than 64 levels. Limits: 10,000 backups / 100,000
            entries. Cancellation waits for the current filesystem call.
          </p>
        </>
      )}
      {error && (
        <p role="alert" className="invalid">
          {error}
        </p>
      )}
    </section>
  );
}
