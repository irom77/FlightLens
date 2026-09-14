# LLM-assisted summaries — implementation plan

Handoff plan for adding optional, key-configured LLM summaries to FlightLens:
an AI summary of the open backup in Inspector mode, and an AI summary of the
differences in Compare mode.

Nothing in this document has been implemented. It is a specification for the
implementing agent, written against `v0.10.1` (`main`). It states the decisions
that are settled, the seams to build, and the checks that must pass.

The prompt to hand the implementing agent is in
[Appendix A](#appendix-a--codex-handoff-prompt), along with a shorter form for
resuming at a later checkpoint.

## 1. Boundary

FlightLens is an offline inspection tool. This feature is the first code path
that can send anything off the machine, so the boundary is part of the
specification, not a footnote.

**What stays true after this ships:**

- Parsing, inspection, comparison, audit and export remain entirely local.
  No LLM result ever feeds back into a parsed value, a derived value, an audit
  verdict or an export.
- Backups remain read-only. The feature reads a `ConfigDocument`; it never
  mutates one.
- The feature is **off by default**. With no key configured there is no button
  to press and no network code reached.
- **No automatic requests.** Opening a backup, switching tabs, switching
  profiles and entering Compare mode never cause a request. A request happens
  only when the user reads a preview of the exact payload and clicks Send.
- Raw CLI text is never transmitted. Only a bounded structured digest is.
- The API key never enters the renderer process and is never written to a log,
  a session file, an error message or an exported summary.

**What becomes false and must be corrected in the documentation:** the README's
claim that there is "no account, upload, telemetry, or background network
connection". See section 14.

This lands inside the commitment already written in `ROADMAP.md` Phase 3:
"Redacted, user-reviewed diagnostic payloads with optional local or cloud
providers. Cloud diagnostics remain opt-in. No network request, credential
storage, configuration mutation, or tuning recommendation happens
automatically."

## 2. Settled decisions

| Decision | Choice | Why |
| --- | --- | --- |
| Payload content | Structured digest only; never raw CLI text | An allowlist of sensitive keys cannot protect an unrecognized vendor key that appears in raw text. A digest is built from what the parser understood, so anything unrecognized is excluded by construction. |
| Key storage | OS keychain via the `keyring` crate | No plaintext on disk. Key stays in the Rust process. |
| Compare payload source | Frontend hands Rust the diff rows it already rendered | The certified-equivalence and "not comparable" logic lives in `src/compare*.ts`. Re-deriving it in Rust would duplicate ~12 modules and risk the summary describing differences the UI declines to call comparable. |
| Transport location | Rust only | The webview CSP (`connect-src ipc: http://ipc.localhost`) already forbids remote origins. Keeping HTTP in Rust means the CSP and `capabilities/main.json` need no change at all. |
| Local model support | In scope for the first release | `ROADMAP.md` Phase 3 already promises "optional local or cloud providers". The OpenAI-compatible adapter reaches Ollama/LM Studio for the cost of URL validation. |
| Summary persistence | In-memory cache only; never written to `.flightlens` sessions | Sessions are reference-only and hash-checked. A cached summary would go stale silently against an edited backup. |
| Summary export | Markdown with a provenance header | A saved file separated from the app must not read as a FlightLens finding. |

## 3. Architecture

Two layers. The network and the secret live in the shell; everything testable
lives in the core, matching how `export.rs` and `feedback.rs` are already
structured.

### 3.1 `crates/flightlens-core/src/llm/` — pure, no network

New module directory. No HTTP client dependency is added to `flightlens-core`;
`cargo test -p flightlens-core` must stay offline and fast.

| File | Responsibility |
| --- | --- |
| `mod.rs` | Public types, re-exports, `PROMPT_VERSION` constant |
| `digest.rs` | `InspectorDigest` and `DiffDigest` construction, caps, label sanitization |
| `prompt.rs` | System instruction and user-message rendering; deterministic |
| `provider.rs` | `Provider`, `Wire`, `build_request()`, `parse_response()` |
| `settings.rs` | `LlmSettings` (non-secret) serde types and validation |

The provider layer is split so it can be fully tested without a socket:

```rust
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>, // never includes the key in Debug output
    pub body: String,                   // serialized JSON
}

pub fn build_request(settings: &LlmSettings, key: Option<&str>, prompt: &Prompt)
    -> Result<HttpRequest, LlmError>;

pub fn parse_response(provider: Provider, status: u16, body: &str)
    -> Result<Summary, LlmError>;
```

`HttpRequest` must implement `Debug` manually, redacting any `Authorization`
or `x-goog-api-key` header value. A derived `Debug` would put the key in a
panic message.

### 3.2 `src-tauri/src/llm.rs` — transport and secrets

New module, registered in `main.rs` alongside `portable`.

Responsibilities: keychain read/write/delete, settings file read/write, the
HTTP call, timeout, single-in-flight guard, and the `#[tauri::command]`
functions. It holds no summarization logic — it calls `build_request`, performs
the transfer, and calls `parse_response`.

### 3.3 Dependencies

Added to `src-tauri/Cargo.toml` only:

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
keyring = "3"
```

`default-features = false` with `rustls-tls` is deliberate: it keeps OpenSSL
and `native-tls` out of the Windows and macOS release builds, which are
cross-compiled by the tag-triggered workflows and must not acquire a system TLS
dependency. Verify both installer workflows still build after this change —
that is a release-blocking check, not a nicety.

`keyring` v3 uses the Windows Credential Manager, the macOS Keychain and the
Linux Secret Service. On a headless Linux or WSL box with no Secret Service
there is no backend; section 8.3 specifies the required fallback.

`flightlens-core/Cargo.toml` gains no new dependencies.

## 4. The digest

Built in Rust from data Rust already owns. Both digest types derive
`Serialize, Deserialize, TS` and are added to `crates/flightlens-core/src/bin/bindings.rs`,
so `pnpm bindings:check` covers them.

### 4.1 `InspectorDigest`

Built from the `ConfigDocument` plus `analysis::inspect(document, rate_profile)`.
Note that `inspect_config` takes only a rate profile; PID-scoped values come from
the document's parameters filtered by the selected PID profile index, so the
digest builder takes both indices and does not assume `Inspection` carries the
PID selection.

Included:

- `firmware`: family, version, pack identifier, and whether the import showed
  `dump all` evidence. A summary written against a `diff` backup must be able
  to say so.
- `profiles`: selected rate profile and PID profile index.
- `parameters`: for each in-scope setting — `semanticKey`, rendered value,
  `unit`, `scope`, `provenance` (`declared` / `derived` / `unknown`), and
  `supported`. **`unknown` must be carried as an explicit state, never omitted
  and never defaulted**; the whole point of the honesty rules is lost if the
  model has to infer absence from a missing field.
- `rates`: per-axis endpoint and midpoint samples from `Inspection::rates`,
  plus each curve's `reason` when unavailable. Not the full point arrays.
- `throttle`: availability and, when available, the same bounded samples.
- `audits`: every `RuleEvaluation` — `id`, `status`, `severity`, `explanation`.
  Including the rules that could not run, with their status, is as important as
  including the findings.
- `diagnostics`: counts by severity. Not the messages, which can quote raw lines.

Excluded by construction: raw `SyntaxLine::raw` text, the source file path, the
document title, the content hash, and every key in the sensitivity list.

### 4.2 `DiffDigest`

Built from rows the frontend supplies. The frontend sends a flat, already-
classified structure; Rust validates, sanitizes and caps it.

```rust
pub struct DiffRowInput {
    pub section: String,      // "parameters" | "features" | "ports" | "modes"
                              // | "rxrange" | "rxfail" | "vtx" | "adjrange"
                              // | "collections"
    pub key: String,          // semantic key or collection/row identifier
    pub scope: String,        // rendered scope label
    pub status: String,       // "changed" | "one_sided" | "conflict"
                              // | "unknown" | "not_comparable"
    pub values: Vec<Option<String>>, // per backup, in slot order; None = unknown
    pub reason: Option<String>,      // why not comparable, when applicable
}
```

Rust-side validation, all of which must be tested:

1. **Reject** any row whose `key` matches `feedback::sensitivity()`.
2. **Reject** unknown `section` or `status` values rather than passing them
   through — an unvalidated string from the renderer must not reach the prompt.
3. **Cap** `values` entries and `reason` at a per-field character limit;
   truncate with an explicit marker.
4. **Cap** total rows (start at 400). When truncating, the digest carries an
   explicit `truncated: { omitted: n, sections: [...] }` field and the prompt
   states that the list is partial. A silently truncated diff would produce a
   summary asserting completeness it does not have.

### 4.3 Label sanitization

Document titles are filenames. Filenames routinely carry a pilot name, a
callsign or a build identity. The existing redaction list covers `set` values
and the `# name:` header — it does not cover this, and this feature is the first
one that would transmit a title.

**Every backup label in both digests is replaced with `Backup A`, `Backup B`,
`Backup C`, assigned in slot order.** The real titles stay in the renderer for
display. The preview dialog shows the mapping locally so the user can read the
summary against their own files, and the mapping is not part of the payload.
The exported summary (section 11) may restore real titles, because that file
stays on the user's disk.

## 5. Sensitivity, shared with feedback

`crates/flightlens-core/src/feedback.rs` already holds `IDENTITY_KEYS`,
`LINK_SECRET_KEYS`, `Category`, and a private `category(key)`.

Promote that to a single shared entry point:

```rust
pub fn sensitivity(key: &str) -> Option<Category>;
```

Keep `category()` as a thin private caller or replace it outright. Existing
`feedback.rs` behavior must not change — its tests are the regression guard.

The digest builders call `sensitivity()` and exclude any match. Add a core test
that walks every key in both lists and asserts that a document containing all of
them produces a digest containing none of them. That test is what keeps a future
added key protecting both paths at once.

## 6. Providers

Four providers, two wire shapes.

| Provider | Base URL | Auth header | Wire |
| --- | --- | --- | --- |
| `gemini` (default) | `https://generativelanguage.googleapis.com` | `x-goog-api-key: <key>` | `Gemini` |
| `openai` | `https://api.openai.com` | `Authorization: Bearer <key>` | `OpenAiCompatible` |
| `openrouter` | `https://openrouter.ai/api` | `Authorization: Bearer <key>` | `OpenAiCompatible` |
| `custom` | user-supplied | `Authorization: Bearer <key>`, omitted when no key | `OpenAiCompatible` |

### 6.1 Gemini wire

`POST {base}/v1beta/models/{model}:generateContent`

```json
{
  "systemInstruction": { "parts": [{ "text": "<system>" }] },
  "contents": [{ "role": "user", "parts": [{ "text": "<user>" }] }],
  "generationConfig": { "temperature": 0.2, "maxOutputTokens": 1024 }
}
```

Success: concatenate `candidates[0].content.parts[*].text`. Handle
`promptFeedback.blockReason` and a `finishReason` other than `STOP` as distinct,
reportable outcomes rather than as an empty summary. Error envelope:
`{ "error": { "code", "message", "status" } }`.

### 6.2 OpenAI-compatible wire

`POST {base}/v1/chat/completions`

```json
{
  "model": "<model>",
  "messages": [
    { "role": "system", "content": "<system>" },
    { "role": "user", "content": "<user>" }
  ],
  "temperature": 0.2,
  "max_tokens": 1024
}
```

Success: `choices[0].message.content`, with `choices[0].finish_reason` checked
for `length`. Error envelope: `{ "error": { "message", "type", "code" } }`.
OpenRouter additionally accepts `HTTP-Referer` and `X-Title`; send
`X-Title: FlightLens` and no referer.

### 6.3 Custom base URL validation

The one place a user-supplied string reaches the network layer, so validate it
in Rust and test the rejections:

- Parse as an absolute URL or reject.
- Scheme must be `https`, **except** that `http` is permitted when the host is
  a loopback address (`localhost`, `127.0.0.0/8`, `::1`). A local model on
  loopback over plain HTTP is fine; a plaintext key to a remote host is not.
- Reject a URL carrying userinfo, a query, or a fragment.
- Strip a trailing slash before joining the path.
- When no key is stored for `custom`, omit the `Authorization` header entirely
  rather than sending `Bearer ` with an empty value. Ollama needs no key.

For the three fixed providers the base URL is a constant and is **not**
user-editable; only `custom` reads a URL from settings.

### 6.4 Model identifiers

Model IDs live in settings with a documented default per provider, not as a
hardcoded literal in the request path.

**The implementing agent must verify the current default model identifiers
against live provider documentation before shipping**, rather than trusting any
constant written in this plan. Gemini Flash and OpenAI's small models have both
been renamed more than once. Note also that newer OpenAI models reject
`max_tokens` in favor of `max_completion_tokens`; confirm which the chosen
default model accepts and handle the rejection with a clear message if it
changes.

## 7. Settings

Non-secret settings, written next to the existing `session.json` in
`app_data_dir()` as `llm.json`, using the same tolerant pattern as
`save_session` — a settings file that cannot be written is reported to the user
here, unlike the session file, because the user explicitly asked to save it.

```rust
pub struct LlmSettings {
    pub enabled: bool,          // false until a key is stored
    pub provider: Provider,
    pub model: String,
    pub base_url: Option<String>, // Some only when provider == Custom
    pub timeout_seconds: u32,     // default 60, clamped 5..=180
}
```

The key is never in this file. On read, reject a settings file whose
`base_url` fails section 6.3 validation rather than trusting a file on disk.

## 8. IPC surface

Commands added to `invoke_handler!` in `main.rs` and to `src/ipc/client.ts`.

### 8.1 Configuration

- `llm_settings() -> LlmStatus` — returns the settings plus `hasKey: bool` and
  `keyHint: Option<String>` (last four characters only). **Never returns the key.**
- `llm_save_settings(settings: LlmSettings) -> LlmStatus`
- `llm_save_key(provider: Provider, key: String) -> LlmStatus` — writes to the
  keychain, clears the local variable promptly.
- `llm_clear_key(provider: Provider) -> LlmStatus`
- `llm_test_connection() -> Result<String, String>` — a minimal request that
  confirms the key and endpoint work, so a user is not debugging credentials
  through a failed summary.

### 8.2 Summaries

- `llm_preview(request: SummaryRequest) -> LlmPreview`
- `llm_summarize(request: SummaryRequest) -> LlmSummary`
- `llm_cancel()`

```rust
pub enum SummaryRequest {
    Inspector { config_id: String, rate_profile: u8, pid_profile: u8 },
    Diff { labels: Vec<String>, baseline: Option<usize>, rows: Vec<DiffRowInput> },
}

pub struct LlmPreview {
    pub system_prompt: String,
    pub user_prompt: String,   // exactly what will be sent
    pub payload_json: String,  // pretty-printed digest, for review
    pub excluded: Vec<Redaction>, // what sensitivity filtering removed
    pub label_map: Vec<(String, String)>, // real title -> "Backup A", local only
    pub bytes: usize,
    pub truncated: Option<Truncation>,
    pub problems: Vec<String>, // why Send is disabled, in dialog field order
    pub destination: String,   // the exact URL the request will go to
}
```

**`llm_summarize` rebuilds the prompt in Rust from the same `SummaryRequest`.
It must not accept prompt text from the renderer.** This is the rule
`file_feedback_report` already follows by rebuilding the issue rather than
opening a URL the page supplies, and the reason is the same: the previewed text
and the sent text must be the same text by construction, not by convention.

`problems` follows the `feedback::problems` precedent — reasons, not a boolean,
so the dialog can say why Send is disabled.

### 8.3 Keychain fallback

When `keyring` reports no available backend (headless Linux, some WSL setups):

- Report it plainly: the platform has no credential store, so FlightLens cannot
  save the key.
- Offer a **session-only key** held in `AppState` in memory, never written to
  disk, discarded on exit, and labeled as such in the settings panel.
- Do not silently fall back to a plaintext file. That would defeat the reason
  the keychain was chosen.

## 9. Prompt

`prompt.rs` renders a system instruction and a user message. Both are
deterministic for a given digest — same input, byte-identical output — so the
preview and the sent payload can be compared in a test.

`PROMPT_VERSION` is a constant bumped whenever the text changes. It appears in
the cache key and in the exported summary's provenance header.

The system instruction must require the model to:

- Treat an `unknown` value as unknown. Never substitute a firmware default,
  never infer a value from a related setting, never describe an unknown as zero.
- Report what the digest does not contain rather than filling the gap.
- Respect a `not_comparable` status — describe it as not comparable, not as
  unchanged.
- State when the digest is truncated or when the backup was a `diff` rather
  than a `dump all`.
- Describe configuration, not flight behavior. A configuration backup does not
  prove how an aircraft flies.
- Avoid presenting tuning changes as instructions.

These mirror the honesty rules the rest of the codebase already enforces
mechanically. Because a model cannot be made to obey them mechanically, the
rendered summary must carry a visible disclaimer in both the panel and the
export (sections 10 and 11).

## 10. UX

### 10.1 Entry points

- **Inspector mode:** an "AI summary" action in the inspector header, beside
  the existing controls. Disabled with a stated reason when not configured.
- **Compare mode:** the same action in the comparison header, enabled only when
  a valid two- or three-backup comparison is selected.
- **Settings:** an "AI summary" section reachable from the sidebar, holding
  provider, model, base URL (custom only), key entry, test connection, and a
  clear statement of what is sent and when.

With no key configured the action states that AI summaries are off and links to
settings. It does not silently disappear — a hidden feature is a support
question.

### 10.2 The preview dialog

Modeled directly on `Feedback.tsx`, which already solves this problem:

1. Destination URL and provider/model, stated plainly.
2. The exact prompt text, scrollable.
3. The digest JSON, scrollable.
4. What sensitivity filtering excluded, and the `Backup A/B/C` label mapping.
5. Payload size, and a truncation notice when applicable.
6. `Send to <provider>` — the only control that causes a request — plus Cancel.

### 10.3 The result panel

- Rendered as **plain text**, not HTML and not `dangerouslySetInnerHTML`. Model
  output is untrusted input to the renderer.
- Footer: provider, model, prompt version, local timestamp, and the disclaimer
  that the text was generated by an external model and is not verified by
  FlightLens.
- Controls: Copy, Save as Markdown, Regenerate, Dismiss.

## 11. Export

Save via the existing dialog pattern in `save_snippet`, writing with
`OpenOptions::create_new(true)` like every other write path in the codebase.
Default extension `.md`.

The file opens with a provenance block, so a summary separated from the app
cannot be mistaken for a FlightLens finding:

```markdown
# FlightLens AI summary

- Generated: 2026-09-14T18:20:11+02:00
- Mode: Inspector
- Backup: quad-4s.txt (SHA-256 3f9a…c21d)
- Profiles: rate 1, PID 1
- Provider: gemini
- Model: <model id>
- Prompt version: 1
- FlightLens: 0.10.1

> This text was generated by an external language model from a redacted
> summary of the backup. It is not produced or verified by FlightLens, and it
> is not a certified finding. Check every claim against the Inspector, Compare
> and Raw views before acting on it.

---

<summary text>
```

For a Compare export, list every backup with its hash and its `Backup A/B/C`
slot. Real filenames are restored here — the file stays on the user's disk.

## 12. Errors, limits, concurrency

- **One request at a time**, using the `IndexPermit` pattern already in
  `main.rs`. A second request reports that one is running.
- **Timeout** from settings, default 60s. A timed-out request reports the
  elapsed time, not a generic failure.
- **Cancellation** via `llm_cancel`, dropping the in-flight request.
- **Error mapping**, in plain language and without the key or the payload:
  `401`/`403` invalid or rejected key; `404` model not found (likely a renamed
  model ID); `429` rate limited; `5xx` provider unavailable; a transport error
  becomes "FlightLens could not reach `<host>`". A `custom` provider that is not
  running should say so — an unreachable loopback address means the local model
  is not started.
- **Never log** the key, the payload, or the response body. `AGENTS.md`:
  "Keep backup contents and secrets out of logs and external services."
- **Cache** in `AppState`, keyed by
  `(document hash or diff row hash, profiles, provider, model, PROMPT_VERSION)`.
  Switching tabs and returning must not re-bill the user. Cleared when the
  document closes.

## 13. Testing

### 13.1 Rust core (`cargo test -p flightlens-core`) — no network

- Digest excludes every key in both sensitivity lists.
- Digest carries `unknown` explicitly and never defaults a missing value.
- Labels are replaced with `Backup A/B/C`; no title appears in serialized output.
- `DiffRowInput` validation: unknown section rejected, unknown status rejected,
  sensitive key rejected, oversized field truncated with a marker, row cap
  produces an explicit `truncated` record.
- Prompt rendering is deterministic and identical between preview and send for
  the same request.
- `build_request` per provider: correct URL, correct auth header, correct body
  shape, and `Debug` output containing no key material.
- Custom URL validation: `https` accepted, remote `http` rejected, loopback
  `http` accepted, userinfo rejected, trailing slash normalized, missing key
  omits the header.
- `parse_response` per provider: success, error envelope, empty candidates,
  truncated `finish_reason`, Gemini `blockReason`, and malformed JSON.

### 13.2 Shell (`src-tauri`)

- Settings round-trip through `llm.json`, including rejection of a stored
  invalid `base_url`.
- `llm_settings` never returns key material; `keyHint` is at most four characters.
- Single-in-flight guard rejects a concurrent request.
- Transport behind a small trait so command-level tests use a fake and the suite
  stays offline.

### 13.3 Frontend (`pnpm test`)

- Diff-row extraction from existing comparison state produces the documented
  `DiffRowInput` shape for two- and three-backup comparisons, preserving
  `not_comparable` and one-sided classifications.
- Settings form validation and the disabled-with-reason states.

### 13.4 Renderer behavior (`pnpm test:ui`)

Playwright runs the browser preview, where `desktop` is false and `api` is
unavailable, so these cover gating rather than transport:

- With no configuration, the AI summary action is present and disabled with a
  stated reason.
- The preview dialog renders prompt, payload and exclusions, and no request is
  issued until Send.

### 13.5 Screenshots

The settings panel, preview dialog and result panel are new UI, so
`pnpm screenshots:check` needs new baselines in both themes. Regenerate with
`pnpm screenshots` and review the diff before committing.

### 13.6 Full gate

`./test.sh` must pass end to end, and `pnpm screenshots:check` must pass
locally before any push. Per `AGENTS.md`, a push is a release.

## 14. Documentation and metadata

Part of the work, not a cleanup pass.

- **`README.md`** — the "Private by default" bullet currently reads "analysis
  runs locally. There is no account, upload, telemetry, or background network
  connection." Rewrite so it stays true: analysis runs locally; there is no
  account, telemetry or background connection; AI summaries are off by default,
  require the user's own API key, send a redacted structured digest and only on
  an explicit Send after the payload is shown; a local model sends nothing off
  the machine. Add a short "AI summaries" section covering setup and what
  leaves the machine.
- **`ROADMAP.md`** — record that the Phase 3 "redacted, user-reviewed
  diagnostic payloads with optional local or cloud providers" commitment is
  partially delivered ahead of the telemetry pipeline, for configuration and
  comparison only. Do not mark Phase 3 items complete.
- **`CHANGES.md`** — an entry under `## Unreleased`, in the same commit as the
  user-visible change.
- **`TODO.md`** — add the deferred items in section 15 and any native checks
  this work leaves unverified. The packaged Windows/macOS keychain path in
  particular cannot be validated on Linux/WSL and must be recorded there.
- **`docs/llm-summaries.md`** — this file, updated to record what was actually
  built and any completion boundary, in the style of
  `docs/portable-sessions.md`.
- **`THIRD_PARTY_NOTICES.md`** — `reqwest`, `keyring` and their new transitive
  license obligations.
- **Version bump** across `package.json`, `src-tauri/tauri.conf.json` and
  `Cargo.toml`, with `Cargo.lock` refreshed via `cargo check`, per `AGENTS.md`
  and `docs/releases/README.md`. This is a new feature, so choose the bump from
  the release policy, not from habit.

## 15. Implementation checkpoints

`AGENTS.md` requires work in checkpoints: stop after each self-contained unit,
state what changed and what was verified, and wait. These are the units.

1. **Core plumbing, no UI.** `llm` module, `LlmSettings`, `Provider`, `Wire`,
   `build_request`, `parse_response`, custom URL validation, the
   `feedback::sensitivity` promotion. Full unit coverage. Verify with
   `cargo test -p flightlens-core` and `cargo clippy --workspace --all-targets`.
2. **Digests and prompt.** `InspectorDigest`, `DiffDigest`, `DiffRowInput`
   validation, label sanitization, caps and truncation, prompt rendering,
   `PROMPT_VERSION`. Regenerate bindings; `pnpm bindings:check` must pass.
3. **Shell, settings and transport.** `src-tauri/src/llm.rs`, keychain with the
   documented fallback, `llm.json`, all commands registered, timeout,
   cancellation, in-flight guard, error mapping, cache. Verify against a real
   provider and against a local Ollama.
4. **Inspector UI.** Settings panel, AI summary action, preview dialog, result
   panel, copy and Markdown export. Frontend and Playwright tests; new
   screenshot baselines.
5. **Compare UI.** Diff-row extraction from existing comparison state, the same
   dialog and panel in Compare mode, multi-backup export. Tests and baselines.
6. **Documentation, version bump and release preparation** per section 14, then
   the full `./test.sh` plus `pnpm screenshots:check` gate.

## 16. Out of scope

Record these in `TODO.md` rather than building them:

- Streaming responses. A single request keeps the preview-then-send contract
  simple; streaming can come later.
- Token accounting and cost display.
- Summaries in `.flightlens` sessions.
- Any LLM involvement in parsing, audit verdicts, derived values or export.
- Blackbox telemetry summaries — that belongs with Phase 3, on the telemetry
  pipeline.
- Prompt customization by the user. A user-edited prompt would invalidate the
  honesty constraints in section 9 and the provenance header's meaning.

## 17. Verification points

Things the implementing agent must confirm rather than take from this plan:

1. **Current model identifiers** for Gemini Flash and the OpenAI default, from
   live provider documentation.
2. **`max_tokens` versus `max_completion_tokens`** for the chosen OpenAI default.
3. **`keyring` v3 API surface** and its behavior when no backend exists.
4. **Both release workflows still build** with `reqwest` + `rustls-tls` added.
   This is release-blocking.
5. **`pnpm format:check`** already fails on pre-existing files per `TODO.md`;
   do not let new files add to that list.

## Appendix A — Codex handoff prompt

Paste the block below into Codex to start the work. It is deliberately
self-contained about process and deliberately thin about design, because the
design is this document and Codex should read it rather than a paraphrase of it.

````
You are implementing a feature in the FlightLens repository at /home/irom/FlightLens
(Tauri v2 + Rust core + React/TypeScript, currently v0.10.1 on `main`).

Read these before writing any code, in this order:

1. AGENTS.md — the repository's binding rules. They override your defaults.
2. docs/llm-summaries.md — the implementation plan for this work. It is the
   specification. Follow it.
3. README.md, ROADMAP.md, TODO.md — the promises the project has already made.
4. docs/releases/README.md — the release policy, needed at the final checkpoint.

TASK

Implement optional, key-configured LLM summaries: an AI summary of the open
backup in Inspector mode, and an AI summary of the differences in Compare mode.
Providers are Gemini (default), OpenAI, OpenRouter, and a custom
OpenAI-compatible base URL for local models such as Ollama.

docs/llm-summaries.md specifies the architecture, the digest shapes, the
provider wire formats, the IPC surface, the UX, the error handling, the tests,
and the documentation updates. Do not redesign any of it. If you find a genuine
problem with the plan, say so and propose the change before implementing it —
do not silently deviate.

HARD CONSTRAINTS

These come from AGENTS.md and section 1 of the plan. Violating any of them
fails the task regardless of whether the code works:

- Raw CLI backup text is never transmitted. Only the structured digest is.
- The API key never enters the renderer process, never reaches a log, an error
  message, a session file, a `Debug` output or an exported summary.
- No request happens without an explicit user Send after the exact payload has
  been shown. Nothing auto-runs on open, tab switch, or profile change.
- `llm_summarize` rebuilds the prompt in Rust from the same request. It must
  not accept prompt text from the renderer.
- Backups stay read-only. No LLM output feeds a parsed value, a derived value,
  an audit verdict or an export.
- Add only the two dependencies the plan names (`reqwest` with
  `default-features = false, features = ["json", "rustls-tls"]`, and `keyring`),
  and only to src-tauri. `flightlens-core` gains no new dependency.
- Do not use Superpowers skills (`superpowers:*`) in this repository.
- Keep changes scoped. No unrelated refactors.
- Do not commit or push unless asked. Do not bump the version before the final
  checkpoint.

WORK IN CHECKPOINTS

Section 15 of the plan defines seven units. Do ONE, then stop and report:

- what changed, as a file list with a one-line reason each
- what you verified, with the command and its actual result
- what you could not verify and why
- what the next unit would be

Then wait. Do not start the next unit on your own. Wait for the user to review
the diff and tell you to continue.

Start with checkpoint 1 (core plumbing, no UI).

VERIFICATION

Run the checks relevant to what you changed, and report anything that could not
run rather than omitting it:

  cargo test -p flightlens-core
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets
  pnpm bindings:check
  pnpm typecheck
  pnpm test
  pnpm test:ui
  ./test.sh                  # full gate, final checkpoint
  pnpm screenshots:check     # after any UI change; needs new baselines

Note that `pnpm format:check` already fails on pre-existing files per TODO.md.
Do not add new files to that list; do not reformat the existing ones.

CONFIRM, DO NOT ASSUME

Section 17 lists what you must check against live sources rather than trust
from the plan: current Gemini Flash and OpenAI model identifiers, whether the
chosen OpenAI model wants `max_tokens` or `max_completion_tokens`, the
`keyring` v3 API and its no-backend behavior, and that both release workflows
still build with rustls. The last one is release-blocking.

Report what you find. If a model identifier in the plan is stale, say so and
use the current one.
````

### Continuing at a later checkpoint

For units 2 through 7, the shorter form is enough, because the plan carries the
detail:

````
Continue the FlightLens LLM summaries work in /home/irom/FlightLens.

Read AGENTS.md and docs/llm-summaries.md first. Checkpoints 1..N are done —
review the working tree to confirm what is actually there rather than trusting
this sentence.

Implement checkpoint <N+1> from section 15 of the plan, and only that
checkpoint. The hard constraints in section 1 and the checkpoint reporting
discipline in AGENTS.md still apply: stop when the unit is done, report what
changed, what you verified with actual command output, and what you could not
verify. Do not commit, push, or bump the version.
````
