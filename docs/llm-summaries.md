# LLM-assisted summaries — implementation plan

Handoff plan for adding optional, key-configured LLM summaries to FlightLens:
an AI summary of the open backup in Inspector mode, and an AI summary of the
differences in Compare mode.

Checkpoints 1–5 are implemented against `v0.10.2`; checkpoint 6 prepares
`0.11.0`. Live-provider and native-platform verification remains pending.
The specification was originally written against `v0.10.1`. The completion
boundary below records the implementation; later sections remain the target
for the remaining checkpoints.

### Checkpoint 6 — final local verification and release preparation — 2026-09-17

Release follow-up: the first tagged macOS build passed Rust checks but exposed
a case-insensitive TypeScript resolution collision between `AiSettings.tsx` and
`aiSettings.ts`. The helper and its test are now named `aiSummarySettings`, and
imports use that distinct basename. Version `0.11.1` carries the correction;
the pushed `v0.11.0` tag remains immutable. Native installer verification is
performed by the new tagged workflows before publishing the milestone notes.

Implementation and documentation cover Inspector and Compare summaries. Version
metadata is now `0.11.0` in package.json, Tauri configuration, workspace Cargo.toml
and the refreshed Cargo.lock: this is a new capability under the release policy.
[Milestone notes](releases/v0.11.0.md) describe compatibility, privacy and remaining
limitations. CHANGES remains Unreleased until an authorized release is cut.
No commit, tag, push or publication was performed.

Verification: `cargo check --workspace` refreshed the lockfile successfully.
The full `./test.sh` passed: Rust formatting and Clippy with warnings denied,
core and 27 shell tests, generated bindings, ESLint, TypeScript, 94 frontend
tests, 37 browser tests and the local real-backup corpus gate.
`pnpm screenshots:check` passed six scenarios covering 20 images.
`pnpm build`, `pnpm format:check` and `git diff --check` passed. The first
formatting run identified App/Inspector integration formatting; those touched
files were corrected and the full formatting check now passes.

Full release validation is **not complete**. These checks need environments or
credentials unavailable in this Linux session; simulated transport is not a
substitute:

1. With a real cloud account and a synthetic backup, review the exact payload,
   Send, verify a nonempty attributed result, regenerate after another review,
   and export. Record provider/model and outcome, never the key or private data.
2. With installed Ollama and a downloaded model, repeat through the loopback
   custom endpoint without a key. Verify cancellation and an unavailable-model
   error. No Ollama executable is installed in this environment.
3. Build both Windows and macOS installers with the locked dependencies. Both
   builds are release-blocking. In packaged apps, verify credential save/restart/
   clear and explicit session-only fallback, clipboard, Markdown save/cancel/
   collision handling, and retained dependency licenses/corresponding source.
4. Before an authorized push, rerun the release gates as needed, date the
   Unreleased changelog section, commit and push with matching `v0.11.0` tag.
   Publish milestone notes only after both installer workflows pass.

Code implementation can be called complete; live integration and release
verification must remain pending until evidence for the checks above exists.

### Checkpoint 5 completion boundary — 2026-09-17

- Compare now shares Inspector's review, Send, plain-text result, Copy,
  Regenerate, Dismiss and Markdown export controls. The action explains missing
  selections and requires a baseline for three backups. The digest covers all
  non-equal comparison rows, independent of table search filters.
- Selected documents/hashes, rate/PID profiles, baseline and settings identify
  the mounted summary. Changes discard pending/late results and displayed text;
  returning to an unchanged request can reuse the Rust cache after review.
- Markdown exports validate the exact cached result against all open documents
  before writing. Compare provenance includes A/B/C titles, full hashes, rate/PID
  selections and baseline. Metadata is escaped and model output stays literal.
- Added browser coverage for two/three slots, selected profiles, reviewed Send,
  export request identity, baseline/profile invalidation and missing selections;
  shell coverage verifies multi-backup provenance and rejects tampered results,
  changed hashes/profiles and missing documents. Added comparison preview/result
  screenshot baselines in both themes.
- README, roadmap, changelog and TODO now reflect both modes. Live provider,
  Ollama and native verification, full final gates and release preparation remain
  checkpoint 6. No version bump, commit, push or live-provider call in this slice.

Verification: all 27 shell tests, 94 frontend tests and 37 browser tests pass.
Workspace Clippy with warnings denied, Rust formatting, TypeScript checking,
ESLint, changed UI/test-file Prettier checks, production build and diff whitespace
checks pass. Four new comparison screenshots were generated and visually reviewed
in both themes; `pnpm screenshots:check` passes all six scenarios (20 images).
The first new browser run found that the fixture lacked a second
PID profile; it now uses the existing Rust multi-profile fixture variant.
No real backup or API key was sent to a provider. Native save dialogs/keychains,
live providers and Ollama remain unverified here.

### Checkpoint 5b — comparison request boundary — 2026-09-17

- Comparison requests now carry document IDs and rate/PID profiles for each slot,
  replacing renderer-supplied labels. Rust resolves open documents, validates
  slot count, document order and profile bounds, and requires a baseline for
  three-backup comparisons. Local label mapping uses trusted document titles.
- Cache/review identity includes every resolved backup hash and the selected
  profiles, slots and baseline. Titles and hashes do not enter provider payloads.
- Rust removes sensitive-key rows using the shared feedback sensitivity list,
  reports them as local preview exclusions, and passes the remaining rows through
  the existing strict digest validation and caps. Core sensitive-key rejection
  remains intact. Exclusion line numbers are zero because diff rows have no
  single source line.
- Compare UI and multi-backup Markdown export remain unimplemented. The new
  request shape provides their document provenance boundary; comparison semantics
  still come from the tested frontend adapter.

Verification: `cargo test -p flightlens` passes 26 shell tests, including two new
comparison boundary tests. Core tests, all 94 frontend tests, workspace Clippy
with warnings denied, Rust formatting, generated bindings, TypeScript checking,
ESLint and `git diff --check` pass. An initial test-fixture type error and a
needless-borrow lint were corrected. No UI changed, so browser/screenshot checks
were not repeated. No live provider, Ollama or native-platform checks ran.

### Checkpoint 5a — comparison extraction — 2026-09-17

- Added `src/summaryDiff.ts`, reusing the existing parameter and collection
  comparison functions, including certified cross-version and three-way gates.
- Values retain original A/B/C slot order regardless of the selected baseline.
  Reasons identify the changed slot for one-sided results. Unknown values remain
  null; declared and derived parameter values retain their provenance.
- Equal rows are omitted. Source-text collection rows carry classification and
  line counts only, with an explicit warning that text differences do not prove
  behavioral differences. Raw CLI lines, source metadata and document identities
  are not serialized by the adapter.
- The adapter is not connected to the UI yet. Rust remains responsible for
  sensitive-key rejection and payload limits; exclusion handling must be wired
  before enabling Compare summaries. Local export must also bind slot IDs and
  profiles to open Rust documents to obtain trusted titles and hashes.
- Six new regression tests cover profile selection, unknowns, provenance,
  certified versus uncertified equivalence, all baselines, explicit false values,
  collection semantics, text-only differences and source-metadata exclusion.
  All 94 frontend tests pass. No provider requests or backup writes occurred.

The remaining work recorded at checkpoint 5a was completed by checkpoints 5b
and 5 above.

### Checkpoint 4 completion boundary — 2026-09-17

- Added sidebar AI settings and a visible Inspector AI summary action, with
  disabled reasons and a settings shortcut. Provider/model/base URL/timeout and
  enablement are saved explicitly; browser preview explains desktop requirements.
  The status badge says “Local inspection” rather than implying model traffic
  is offline. Keys are write-only password inputs cleared on submission, with explicit
  session-only storage and key clearing. Saving settings never sends a request.
- Both summary and connection test dialogs show destination, provider/model,
  exact prompts, expandable digest JSON, local label mapping, sensitivity
  exclusions, payload bytes, truncation and blocking problems. Send is explicit
  and single-use; retry and regeneration require fresh review. Cancellation and
  Inspector identity/profile/settings changes discard late results.
- Model results render as plain text with provider, model, prompt version, local
  timestamp, cache indicator and disclaimer. Copy includes attribution and the
  disclaimer. Dismiss removes the panel; summaries never enter sessions.
- Added `llm_save_summary(request, summary)`: Rust compares the displayed result
  with the exact cached request before exporting its own cached text. Markdown
  includes UTC ISO timestamp, local backup title/full hash, 1-based and CLI
  profiles, provider/model, prompt version and app version. Metadata is escaped;
  model output uses an indented literal block, preserving plain-text rendering
  even for HTML, Markdown image links or embedded fences. The existing
  `save_new` dialog uses `create_new` and retries collisions. Evicted or changed
  results require a fresh reviewed summary before saving.
- Compare row extraction, entry points and multi-backup export remain checkpoint 5.
  No dependency, CSP/capability, version, commit or release changes in this slice.

Verification: `cargo test -p flightlens` passes all 24 shell tests, including
cached-export provenance/tamper checks and UTC date boundaries. `pnpm test`
passes 88 tests in 22 files. The full `pnpm test:ui` run passes 35 tests; the
expanded AI suite covers five scenarios, including no automatic requests,
profile invalidation, preview problems, credential entry/clearing, connection
review, literal output, copying/export, cancellation and retry.
`cargo clippy --workspace --all-targets -- -D warnings`, Rust formatting,
`pnpm bindings:check`, `pnpm typecheck`, `pnpm lint`, `pnpm build`, changed/new
component Prettier checks and `git diff --check` pass. `pnpm screenshots` and
`pnpm screenshots:check` pass all four scenarios with 16 baselines, including
six new settings/preview/result images in both themes. Baselines were visually
reviewed; screenshots use a fixed locale/timezone for timestamp stability.
The first targeted browser run found an ambiguous test label, corrected without
weakening assertions. No provider network calls occurred in these checks.

Live provider/Ollama and native installer, keychain and save-dialog checks remain
pending. The final release `./test.sh` gate remains checkpoint 6; this slice ran
the relevant shell, frontend, browser, build and screenshot checks directly.

### Checkpoint 3 completion boundary — 2026-09-16

- Added desktop IPC and Rust transport, settings validation/read/write in
  `llm.json`, native credential storage, explicit session-only keys, cancellation,
  one-request guard, static error mapping and bounded in-memory summary caching.
  No UI or Markdown summary export is implemented yet.
- `keyring` 3.6.3 requires explicit backend features: apple-native,
  windows-native, sync-secret-service, crypto-rust and vendored. Without platform
  features it uses a mock store, per the [upstream documentation](https://docs.rs/keyring/3.6.3/keyring/).
  Linux uses vendored D-Bus and Rust crypto. `reqwest` 0.12.28 uses rustls with
  default features disabled. No new core dependencies were added.
- The [HTTP client](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html)
  disables redirects, automatic retries and environment proxies. Thus a loopback
  model request cannot be routed through an environment proxy. Corporate proxies
  are not supported in this first implementation. Response reads stop at 1 MiB;
  error status bodies are discarded, and transport/keychain errors are mapped
  without logging or exposing backend messages.
- Settings and credential IO use blocking workers. Stored keys never return to
  the renderer. Key entry is a write-only IPC argument, transiently present in
  the settings renderer; the contradictory literal “never enters the renderer”
  wording is interpreted as applying to stored keys, as required by the planned
  key-entry API. Owned Rust key buffers are cleared promptly, without claiming
  guaranteed erasure of OS, IPC or allocator copies. Hints are at most four
  characters and are omitted for keys of four characters or fewer.
- Credential failures are reported, never silently persisted in plaintext.
  `llm_save_key` takes an explicit `sessionOnly` flag. A loopback custom endpoint
  can work without a credential-store backend; remote custom endpoints fail
  closed when the credential store is inaccessible. Session overrides last only
  until clear/replacement or app exit. Clearing removes the session override and
  attempts deletion of the stored key, reporting any failure.
- Send rebuilds the prompt from its request and requires a matching, single-use
  Rust-held preview receipt. The receipt includes provider/model/endpoint,
  timeout, prompt version and complete local request identity. Settings/key edits,
  cancellation and document closure invalidate pending review. Connection tests
  now have `llm_test_preview` and likewise require review before their send command;
  their fixed payload contains no backup data and can run with summaries disabled.
- Cache identity uses exact local request/prompt equality rather than a digest
  hash, avoiding collisions or a new hashing dependency. It includes Inspector
  document hash/profiles and Compare input rows, slots and baseline, plus settings
  including endpoint. Cache is bounded to 16 entries/8 MiB and cleared on any
  document closure or settings/key edit. No summaries enter sessions. A
  `regenerate` flag bypasses cached output after a fresh preview. `generatedAt`
  carries Unix seconds as a decimal string for local formatting in the next UI
  checkpoint. Comparison IPC input is limited to 2 MiB before digest construction.
- Registered commands and typed client wrappers. Generated core bindings include
  non-secret status, request, preview and summary contracts. Added shell tests to
  `test.sh` and both native release workflow gates; no workflows were dispatched.
- Updated privacy documentation and [dependency/license inventory](llm-dependencies.md).
  Native installer contents and associated notice bundles remain release checks.

Verification: `cargo test --workspace` passes 147 tests (124 core and 23 shell,
including nine new shell tests). `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, `pnpm bindings:check`, `pnpm typecheck`, `pnpm lint`,
changed-client Prettier checking and `git diff --check` pass. `pnpm test` passes
all 86 tests in 21 files when run alone; the first run alongside Rust builds
passed its assertions but hit three Vitest worker RPC timeouts. No test or
runner configuration was weakened. The shell transport tests use synthetic
loopback HTTP responses; no cloud endpoint is contacted. Cross-target dependency
graphs for Windows MSVC and macOS ARM64 contain rustls and no OpenSSL/native-tls;
this is dependency verification, not native compilation or installer validation.
UI/Playwright and screenshot gates remain for the UI checkpoints.

Live provider/Ollama tests and native Windows/macOS installer/keychain tests are
pending: this environment has only the Linux target and no Ollama executable.
No real backup or key has been sent to any provider during this implementation.

### Checkpoint 2 completion boundary — 2026-09-16

- Added Inspector and Compare digest builders and prompt version 1, with generated
  TypeScript declarations for the digest and settings types. No shell or UI yet.
- Inspector enumerates the matching certified schema, excluding sensitive keys
  and unrecognized settings. Missing and invalid values carry `value: null` and
  `provenance: unknown`; only existing parser recovery supplies derived values.
  Rate and PID selections are independent. Without a matching pack the parameter
  list is empty; no unsupported setting values are forwarded.
- Inspector includes five evenly spaced curve samples (including endpoints,
  center and half-stick positions), unavailable reasons, recovery inputs, all
  audit evaluations and fixed-severity diagnostic counts. Source text, paths,
  titles, hashes, diagnostic messages and OSD identity previews are excluded.
- Compare accepts two or three slots and an optional validated baseline index.
  It rejects sensitive keys, unknown sections/statuses and incorrect value counts,
  including rows beyond the cap. It preserves slot order and null unknown values.
  Only slot counts enter the builder; local filenames cannot become labels.
- Compare retains at most 400 rows and reports omitted counts and sorted sections.
  Values, reasons, keys and scope labels are capped at 512 Unicode characters,
  including an explicit truncation marker. Inspector setting values use the same
  field cap; parameter cardinality is bounded by the bundled schema.
- Prompts treat payload fields as untrusted data, preserve unknown/derived and
  not-comparable distinctions, and state evidence and truncation limitations.
  Rendering is byte-identical for rebuilt inputs. Actual preview/send IPC equality
  remains a shell-checkpoint responsibility. Builders, rather than deserialized
  digest objects supplied by the renderer, must be used by those commands.

Verification: `cargo test -p flightlens-core` passes all 124 tests, including
seven digest integration tests and the shared-list digest regression test.
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`pnpm bindings:check`, `pnpm typecheck` and `git diff --check` pass.
Frontend behavior, screenshots, live providers and native keychain checks are
not exercised in this core-only checkpoint.

### Checkpoint 1 completion boundary — 2026-09-16

- Added the pure `llm` core module: provider/wire types, non-secret settings,
  disabled defaults, timeout clamping, request preparation and response parsing.
  Promoted `feedback::sensitivity` without changing feedback redaction behavior.
- No dependencies, sockets, key storage, IPC commands or UI have been added.
  `Prompt` is only the transport input type for now; digest construction,
  deterministic prompt rendering and generated bindings belong to checkpoint 2.
- Requests hide all headers, URLs and bodies in `Debug`; prompts and summaries
  also hide their text. Errors never include provider-supplied messages. Blocked,
  truncated, incomplete and empty outputs are separate outcomes.
- To preserve the no-new-core-dependencies rule, custom URLs use a strict ASCII
  parser: DNS names, canonical IPv4, bracketed IPv6, optional ports and simple
  endpoint paths. Encoded/Unicode hosts, numeric IPv4 shorthand, credentials,
  queries, fragments and ambiguous paths are rejected. Use ASCII/punycode names
  and unencoded endpoint prefixes. The shell must disable redirects and bound
  response reads; the pure parser additionally limits bodies to 1 MiB.
- Custom settings start with `http://localhost:11434` and an empty model field;
  the user must name an installed model before validation succeeds. No local
  model is assumed to be installed.

Verification: `cargo test -p flightlens-core` passes all 116 tests (including
14 provider/settings integration tests and the shared sensitivity-list test);
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
and `pnpm bindings:check` pass. Live requests, credentials and native installer
checks are deferred to the transport/release checkpoints. No backup or key was
sent to any provider during this work.

Provider documentation checked on 2026-09-16:

- Gemini defaults to `gemini-3.8-flash`, listed as stable in the
  [model catalog](https://ai.google.dev/gemini-api/docs/models); the request uses
  the documented [generateContent wire](https://ai.google.dev/api/generate-content).
- OpenAI defaults to the non-reasoning
  [`gpt-4.1-mini`](https://developers.openai.com/api/docs/models/gpt-4.1-mini).
  The [Chat Completions reference](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)
  still documents `max_tokens`, deprecated in favor of `max_completion_tokens`
  and incompatible with o-series models. The adapter retains the plan's
  `max_tokens`/temperature shape for this default; arbitrary model overrides
  may reject these parameters and receive a fixed explanatory error. Actual
  account/model availability still needs checkpoint 3's live smoke check.
- OpenRouter defaults to
  [`openai/gpt-4.1-mini`](https://openrouter.ai/openai/gpt-4.1-mini).
  Its [attribution documentation](https://openrouter.ai/docs/app-attribution)
  confirms the plan's `X-Title` is still accepted; no referer is sent.

Remaining UI interpretation: section 10's visible, disabled action with a reason
will take precedence over section 1's hidden-action wording. The key-entry
interpretation and explicit preview receipt are recorded in checkpoint 3 above.


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
- `llm_save_key(provider: Provider, key: String, session_only: bool) -> LlmStatus` — writes to the
  keychain, clears the local variable promptly.
- `llm_clear_key(provider: Provider) -> LlmStatus`
- `llm_test_preview() -> LlmPreview` — previews the fixed no-backup test payload.
- `llm_test_connection() -> Result<String, String>` — a minimal request that
  confirms the key and endpoint work, so a user is not debugging credentials
  through a failed summary.

### 8.2 Summaries

- `llm_preview(request: SummaryRequest) -> LlmPreview`
- `llm_summarize(request: SummaryRequest, regenerate: bool) -> LlmSummary`
- `llm_cancel()`

```rust
pub enum SummaryRequest {
    Inspector { config_id: String, rate_profile: u8, pid_profile: u8 },
    Diff { slots: Vec<SummarySlot>, baseline: Option<usize>, rows: Vec<DiffRowInput> },
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
