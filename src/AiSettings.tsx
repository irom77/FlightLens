import { useRef, useState } from "react";
import type {
  LlmPreview,
  LlmSettings,
  LlmStatus,
  Provider,
} from "./bindings/core";
import { api, desktopAvailable } from "./ipc/client";
import { Modal } from "./Modal";
import { AiPreview } from "./AiPreview";
import { providers, providerSettings, settingsProblem } from "./aiSettings";
import { message } from "./errorMessage";

export function AiSettings({
  status,
  onChange,
  onClose,
}: {
  status: LlmStatus | null;
  onChange: (status: LlmStatus) => void;
  onClose: () => void;
}) {
  const [settings, setSettings] = useState<LlmSettings>(
    status?.settings ?? providerSettings("gemini"),
  );
  const [key, setKey] = useState("");
  const [sessionOnly, setSessionOnly] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [preview, setPreview] = useState<LlmPreview | null>(null);
  const alive = useRef(true);
  const testEpoch = useRef(0);
  const close = () => {
    alive.current = false;
    setKey("");
    onClose();
  };
  const dirty = JSON.stringify(settings) !== JSON.stringify(status?.settings);
  const problem = settingsProblem(settings);
  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
    } catch (e) {
      if (alive.current) setError(message(e));
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  const update = (s: LlmStatus) => {
    onChange(s);
    if (alive.current) setSettings(s.settings);
  };
  return (
    <>
      <Modal labelledBy="ai-settings-title" onClose={close}>
        <div className="section-heading">
          <h2 id="ai-settings-title">AI summary settings</h2>
          <button aria-label="Close AI settings" onClick={close}>
            ×
          </button>
        </div>
        <p>
          Optional and off by default. Inspection stays offline. Only an
          explicit Send after payload review sends a redacted structured digest
          to your chosen provider. Raw backup text is never sent. A loopback
          model keeps the request on this machine.
        </p>
        {!desktopAvailable() && (
          <p className="notice">AI settings require the desktop app.</p>
        )}
        <fieldset
          className="ai-settings-fields"
          disabled={busy || !desktopAvailable()}
        >
          <label className="checkbox">
            <input
              type="checkbox"
              checked={settings.enabled}
              onChange={(e) =>
                setSettings({ ...settings, enabled: e.target.checked })
              }
            />
            Enable AI summaries
          </label>
          <label>
            Provider
            <select
              value={settings.provider}
              onChange={(e) => {
                setKey("");
                setSettings({
                  ...providerSettings(e.target.value as Provider),
                  enabled: settings.enabled,
                });
              }}
            >
              {Object.entries(providers).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Model ID
            <input
              value={settings.model}
              maxLength={200}
              onChange={(e) =>
                setSettings({ ...settings, model: e.target.value })
              }
            />
          </label>
          {settings.provider === "custom" && (
            <label>
              Base URL
              <input
                value={settings.baseUrl ?? ""}
                maxLength={2048}
                placeholder="http://localhost:11434"
                onChange={(e) =>
                  setSettings({ ...settings, baseUrl: e.target.value })
                }
              />
              <small>
                Use an installed model ID. HTTPS is required except on loopback.
              </small>
            </label>
          )}
          <label>
            Timeout (seconds)
            <input
              type="number"
              min={5}
              max={180}
              value={settings.timeoutSeconds}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  timeoutSeconds: Number(e.target.value),
                })
              }
            />
          </label>
          {problem && <p className="notice">{problem}</p>}
          <button
            disabled={Boolean(problem) || !dirty}
            onClick={() =>
              void run(async () => {
                update(await api.llmSaveSettings(settings));
                setNotice("Settings saved. No request was sent.");
              })
            }
          >
            Save settings
          </button>
          {dirty ? (
            <p>
              Save settings before editing the key or testing the connection.
            </p>
          ) : (
            <>
              <p>
                {status?.hasKey
                  ? `${status.sessionOnly ? "Session-only key (discarded on exit)" : "Key in OS credential store"}${status.keyHint ? ` · ending ${status.keyHint}` : ""}`
                  : "No key configured."}{" "}
                {settings.provider === "custom" &&
                  "A local endpoint may work without a key."}
              </p>
              {status?.credentialProblem && (
                <p className="notice">{status.credentialProblem}</p>
              )}
              <label>
                New API key
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  value={key}
                  maxLength={4096}
                  onChange={(e) => setKey(e.target.value)}
                />
              </label>
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={sessionOnly}
                  onChange={(e) => setSessionOnly(e.target.checked)}
                />
                Use a session-only key; discard on exit
              </label>
              <div className="actions">
                <button
                  disabled={!key || /[^\x21-\x7e]/.test(key)}
                  onClick={() => {
                    const entered = key;
                    setKey("");
                    void run(async () => {
                      update(
                        await api.llmSaveKey(
                          settings.provider,
                          entered,
                          sessionOnly,
                        ),
                      );
                      setNotice("Key saved.");
                    });
                  }}
                >
                  Save key
                </button>
                <button
                  onClick={() =>
                    void run(async () => {
                      try {
                        update(await api.llmClearKey(settings.provider));
                        setNotice("Key cleared.");
                      } catch (error) {
                        // A failed native deletion may still remove the session override.
                        await api
                          .llmSettings()
                          .then(update)
                          .catch(() => {});
                        throw error;
                      }
                    })
                  }
                >
                  Clear key
                </button>
                <button
                  onClick={() =>
                    void run(async () => {
                      const p = await api.llmTestPreview();
                      if (alive.current) setPreview(p);
                    })
                  }
                >
                  Review connection test
                </button>
              </div>
            </>
          )}
        </fieldset>
        {error && (
          <p className="notice" role="alert">
            {error}
          </p>
        )}
        {notice && <p role="status">{notice}</p>}
        <div className="actions">
          <button onClick={close}>Done</button>
        </div>
      </Modal>
      {preview && (
        <AiPreview
          preview={preview}
          settings={settings}
          onClose={() => {
            testEpoch.current++;
            setPreview(null);
          }}
          send={async () => {
            const current = testEpoch.current;
            const result = await api.llmTestConnection();
            if (alive.current && current === testEpoch.current) {
              setPreview(null);
              setNotice(result);
            }
          }}
        />
      )}
    </>
  );
}
