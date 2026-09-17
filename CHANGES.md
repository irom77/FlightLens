# Changes

User-visible changes to FlightLens, newest first. Each released section matches
the `v<version>` tag whose build produced the Windows and macOS installers.
Work that is committed but not yet released sits under `## Unreleased` and is
renamed to the version when the release is cut. Purely internal work
(formatting, comments, test-only refactors) is left out.

## 0.11.1 — 2026-09-17

- Fix AI settings module resolution on case-insensitive Windows/macOS filesystems,
  which blocked the 0.11.0 installer builds. AI summary behavior is unchanged.

## 0.11.0 — 2026-09-17

- Add the core and desktop command foundation for optional AI summaries:
  bounded redacted digests, deterministic prompts, provider settings, OS
  credential storage or explicit session-only keys, reviewed-payload gating,
  cancellable Rust HTTP transport and memory-only caching. The feature is off
  by default. Inspector now includes settings, exact-payload preview, plain-text
  results, copy with attribution and safe Markdown export with backup provenance.
  Connection tests and regeneration also require a reviewed Send. Compare now
  supports two- and three-backup summaries with baseline/profile invalidation and
  Markdown provenance for every slot. Its row adapter preserves comparison
  classifications and omits raw collection source text. Comparison previews
  filter sensitive rows and resolve backup labels and hashes from open documents.
  The status badge now says “Local inspection” to distinguish offline analysis
  from explicitly requested model traffic.

## 0.10.2 — 2026-09-16

- Recover omitted rate-profile settings for source-verified Betaflight
  `4.5.3.KAACK_V19` and `2025.12.3-alpha.KAACK_V19` backups with a reset
  baseline. Rate curves can use these values, labeled with the
  exact vendor version; recovered settings are never exported. Other vendor
  identities remain unverified. Ambiguous reset, rate-profile selector or
  assignment commands withhold rate recovery until a new valid reset.

- Recover ten omitted PID gains (P/I/F on all axes and yaw D) from verified
  official Betaflight 4.5.0–4.5.5 defaults after a valid reset. PID inspection
  labels recovered values and comparisons include them; export still requires
  explicit declarations. Roll/pitch D, D-min/D-max, unverified releases and
  ambiguous reset/profile/tuning sequences keep missing values unknown.

- Import compact inspection documents and fetch Raw source in pages of at most
  500 lines, keeping full source in the backend. Source jumps wait for their page;
  stale requests after navigation/reload/close cannot replace the current source.
  Failed source requests show an error with a retry action.

- Reuse immutable backend document snapshots during analysis, export, feedback
  and session validation instead of copying full backups on every request.
  Imports move the original document into backend storage without a full clone.
  In-flight requests retain their snapshot when a document closes or reloads.

- Add an offline synthetic document-cost probe and IPC assessment for maintainers,
  identifying payload expansion and measuring a tested compact source-evidence
  projection for future raw paging. Comparisons now support the compact document
  view, with a generated Rust/TypeScript contract and cross-language regression
  coverage consuming serialized Rust views from synthetic inputs.
  The probe compares full and compact import payloads and parsed JS heap.
  Frontend tests now require Cargo
  (provided by test.sh and CI).

- Keep in-memory artifacts and document metadata together in the reactive
  workspace store, removing App's manual render counter and making document
  replacement and removal a single state update.

- Split the inspector and its tab views out of App.tsx into separate modules for
  maintenance, preserving the existing interface and navigation behavior.

- Show source-verified mode names for 30 official Betaflight releases and both
  supported KAACK builds, including camera controls, VTX pit mode, user switches
  and launch control. Labels follow the release; unknown IDs remain numeric.

- Snippet and portable-session saves now suggest unused filenames and reopen the
  save dialog with an explanation after a filename collision. Existing files
  remain protected; session references follow the final destination folder.

- Add a developer verification tool and pinned upstream evidence for PID-default
  research across Betaflight 4.5.0–4.5.5, including the CLI diff omission path
  and platform include order, plus a source-checked C harness for default and
  slider/mode tuning arithmetic and the complete SITL PID-profile reset through
  the upstream dispatcher with a PID-only host registry under
  O0/O2/Ofast. A hardware preprocessing tool verifies a pinned SPEEDYBEEF405V4
  configuration against 44 public upstream inputs; the verified PID recovery
  scope is described above.

- Add JavaScript/TypeScript and React hook lint checks to local validation,
  branch/PR CI, and both installer release workflows.

- Add expandable sample-value tables to rate, throttle, and filter plots,
  accessible by keyboard and touch with explicit input/output units.

- Normalize workspace page requests to 50-entry boundaries, including callers
  that supply an offset between pages.

- Bound retained file-source registrations to 4,096 and queued native drops to
  256, with an error when full. Duplicate queued drops reuse their existing entry;
  queued imports and open documents keep their source IDs.

- Reuse file-source registrations on repeated imports and release them when all
  associated documents close, while preserving references for older snapshots.

- Show throttle previews for verified official Betaflight 4.2.1–4.2.11, 4.3.1,
  and 4.4.1–4.4.3 releases when the selected profile declares valid inputs.

- Point audit references at the matched Betaflight schema tag, label them as
  schema references, and omit them when no schema matches. Vendor build suffixes
  no longer produce nonexistent upstream tags.

- Run Windows/macOS checks on branch pushes and pull requests, and require Rust
  formatting and warning-free Clippy checks before building release installers.
  Local `./test.sh` now includes workspace Clippy checks.

- Keep keyboard focus inside paste and feedback dialogs, support Escape dismissal,
  and return focus to the opener when closed. Prevent the paste shortcut from
  opening another dialog over an existing modal.

- Wait for a pause in typing before preparing feedback previews, avoiding repeated
  backup redaction during a typing burst while keeping stale previews unfilable.
- Give feedback reports that exceed the encoded URL limit a calculated shortening
  instruction, including for non-Latin descriptions.

## 0.10.1 — 2026-09-14

- Move session controls from the sidebar into the main area in inspection and comparison. Keep Open and Save visible, with recovery controls under Session options.

## 0.10.0 — 2026-09-14

- Fix empty throttle graphs for ProSpec on `4.5.3.KAACK_V19` and ProSpec2 on
  `2025.12.3-alpha.KAACK_V19`, including the latter’s hover-dependent curve and
  read-only hover setting with source navigation.

- Support throttle previews for official Betaflight 4.3.2 backups with explicit
  throttle settings, using verified upstream calculations.

- Keep the Betaflight dump-processing guide as the next TODO task; defer broader
  export/schema coverage, formatting cleanup and security CI work. Record the
  existing out-of-range `osd_units` diagnostic as an accepted limitation.

- Postpone Phase 2 items 7 and 8 (native file routing and packaged platform smoke
  validation) to TODO. Mark Phase 2 complete within the revised items 1–6 scope;
  deferred native checks remain unfinished and release checks remain required.

## 0.9.0 — 2026-09-13

- Remove the planned MIT relicensing and installer-signing tasks. The existing
  GPL-3.0-or-later license and unsigned distribution remain unchanged.

- Complete Phase 2 item 6 locally with a documented reference-only session
  contract, acceptance evidence, and explicit pasted/Blackbox input limits.
  Native OS file routing and packaged platform validation remain separate items.

- Allow portable sessions to save file-backed recognized iNav and ArduPilot text
  backups, including their active selection and original content hashes. They
  remain recognition-only; pasted backups and Blackbox imports are still excluded.

- Add Relink session backups to locate moved files through native file pickers.
  Only identical contents are accepted; saved profiles and comparison selections
  restore, and Save As records replacement locations without changing backups.

- Add Retry session to reread the last confirmed portable session without a file
  picker, repeating reference confirmation and hash checks before restoring its
  saved selections. Cancelled or invalid opens keep the previous retry target.

- Retain unavailable active-backup and comparison selections, including graph
  order and baseline, through portable-session Save As. A checkbox lets users
  save current selections instead, and a new available comparison takes
  precedence over the retained comparison.

- Keep an unavailable session workspace folder when saving a new session, with
  its path adjusted for the new location. Choosing a workspace folder replaces
  the retained location; cancelling or refreshing leaves it intact. The restore
  message explains which folder Save As will retain.

- Preserve failed backup references from the last confirmed portable session
  during the app run and carry them into Save As with rebased paths and original
  hashes/profiles. Conflicting references or incompatible foreign paths block
  saving instead of silently losing references. Reopen a saved session to retry.

- Save and restore two- or three-backup comparison order, baseline, and each
  backup's selected rate/PID profiles in portable sessions. Profile selections
  now persist across inspector/comparison navigation and are shared by both
  views. Unavailable saved profiles produce explicit fallback messages; a
  comparison with missing or duplicate backups is not silently substituted.

- Add native Save Session As and Open Session controls for file-backed CLI
  documents, active document, tab and theme. Save references to a new file;
  confirm referenced locations before opening matching backups. Report missing
  or changed backups individually and preserve existing open documents. Restore
  the saved workspace folder with bounded metadata indexing after confirmation;
  unavailable folders preserve the current explorer.

- Document and implement the version-1 portable session format foundation, with
  bounded metadata validation and relative-path resolution.

- Add a recursive, metadata-only workspace explorer with file/path search,
  50-file pages, on-demand opening, cancellable background batches and refresh
  after files change or drives reconnect. Index one selected folder without
  opening its backups; report skipped entries and scan limits (10,000 backups,
  100,000 directory entries and 64 levels). Links are skipped. Workspace folder
  selection can be retained in portable sessions.

- Add optional third-backup selection with angular velocity and throttle overlays
  and independent profiles. Add an explicitly selected baseline for three-way
  parameter comparison, separating one-sided changes, matching changes and
  conflicts while retaining unknown values, compatibility gates and source links.
  Apply the same baseline to features, ports, modes, receiver ranges/failsafes,
  VTX tables/activations, adjustment ranges and CLI collection text. Preserve
  declaration normalization and resets; label text-only differences separately
  from semantic equivalence.

- Overlay both selected backups’ throttle curves in Compare, using independent
  rate profiles, shared percentage axes, source labels and hover values. Show
  unavailable reasons per backup using the existing verified throttle mapping.
- Expand Phase 2’s three-backup comparison scope to require shared angular
  velocity and throttle graphs for both two and three selected CLI backups.

## 0.8.0 — 2026-09-12

- Add a read-only Throttle Curve Preview beside angular velocity in the Rates
  tab, with imported MID, EXPO and limit values, source-line links, and normalized
  input/throttle-command percentage axes. Support verified official Betaflight
  4.2.0, 4.3.0, 4.4.0 and 4.5.0–4.5.5; missing or invalid inputs, vendor builds,
  other releases and the unsupported MID=100 edge show an unavailable reason.
  Validate calculations against 28,512 compiled upstream C reference vectors.
  The separate 2025.12 hover model remains deferred.

- Prioritize Phase 2.5's Throttle Curve Preview before resuming Phase 2 with
  three-backup comparison; retain the completed comparison coverage boundary.

## 0.7.0 — 2026-09-12

- Define the completed Phase 2 comparison coverage boundary and defer broader
  equivalence research so three-backup comparison is the next feature priority.

- Compare virtual-current scale and offset across verified official Betaflight
  4.5.0–4.5.5 and 2025.12.1–2025.12.5 releases, enforcing scale bounds of
  -16000–16000 and offset bounds of 0–16000 centiamperes (0.01 A).

- Document virtual-current scale and offset mapping prerequisites, with an upstream
  verifier covering 11 releases and explicit units and runtime limitations.

- Compare ADC current offset across verified official Betaflight 4.5.0–4.5.5
  and 2025.12.1–2025.12.5 releases, enforcing signed bounds of -32000–32000 mA.

- Document current calibration mapping prerequisites with an upstream verifier
  covering 11 releases; identify the signed ADC offset candidate and defer hardware scale.

- Document Codex co-author attribution for future assisted commits.

- Compare voltage divider and multiplier settings across verified official
  Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5 releases, enforcing bounds of 1–255.

- Document voltage calibration comparison prerequisites with an upstream verifier
  covering 11 releases; distinguish global divisor settings from hardware scale.

- Compare battery capacity and forced cell count across verified official
  Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5 releases.

- Document capacity and forced-cell-count comparison scope, with an upstream
  verifier covering 11 official Betaflight releases and reviewed runtime differences.

- Compare final explicit adjustment-range slots on matching verified official
  Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5 versions, with normalized
  ranges, optional defaults, filtering and source lines.

- Document adjustment-range comparison prerequisites, with a reproducible upstream
  verifier covering 11 official Betaflight releases.

- Compare final explicit VTX activation slots on matching verified official
  Betaflight 4.5.0–4.5.5 versions, with normalized ranges, conservative selector
  prerequisites, filtering and source lines.

- State the GitHub account requirement before the feedback form and label the
  browser hand-off “Continue to GitHub”.

## 0.6.0 — 2026-09-12

- Compare explicit VTX table dimensions, bands and power arrays on matching verified
  official firmware, with command ordering, unknown dependencies, filtering and source lines.

- Define a conservative official 4.5 VTX activation comparison scope; document
  selector dependencies and why 2025.12 requires additional build evidence.

- Document VTX table and activation comparison prerequisites, with a reproducible
  upstream verifier covering 11 official releases.

- Compare final explicit receiver failsafe modes and normalized set values on
  matching verified official firmware, with channel filtering and source lines.

- Document the verified receiver failsafe comparison scope and add a developer
  verifier for upstream semantics across 11 official releases.

- Fix certified cross-version parameter comparisons showing “Not comparable”
  because the bundled firmware-family identifier differed from parsed backups.

- Prevent feedback submission from using a stale preview after fields or attachment
  consent change. Count the complete report and configuration when deciding
  whether the backup must be attached as a file.

- Send feedback from inside the app: a subject, a description, and optionally
  the open configuration. FlightLens opens a prefilled GitHub issue in your
  browser and copies the configuration to your clipboard for you to paste; it
  transmits nothing itself and needs no account or server. Names and radio link
  identities are replaced before copying, and everything that leaves the app is
  shown for review first.

- Compare final explicit `rxrange` endpoints by channel on matching verified
  official 4.5.0–4.5.5 and 2025.12.1–2025.12.5 firmware versions, with filtering
  and source lines. Resets and unverified syntax clear earlier knowledge; missing
  ranges remain unknown. Vendor and cross-version comparisons remain uncertified.

- Compare preserved CLI collection text for `vtxtable`, `vtx`, `rxfail`,
  `rxrange`, and `adjrange`, with command filtering and source lines. Results
  describe text differences only; missing groups remain unknown.

- Compare same-version mode assignments by slot, including mode IDs, channels,
  ranges, logic, and linked IDs, with filtering and source lines. Missing slots
  and omitted logic/link fields stay unknown; cross-version assignments remain
  not comparable.

- Compare same-version serial-port declarations by identifier, function mask,
  and all four baud settings, with filtering and source lines. Missing ports stay
  unknown; cross-version allocations remain not comparable.

- Compare final explicit feature declarations between backups on the same
  firmware version, with enabled/disabled states, source lines, filtering, and
  hidden equal values. Missing declarations stay unknown; cross-version feature
  declarations remain not comparable.

- Compare `motor_poles` and the maximum, minimum, and warning battery cell-voltage
  thresholds across verified official Betaflight 4.5.0–4.5.5 and
  2025.12.1–2025.12.5 releases. Other cross-version settings remain uncertified.

- Validate GPS rescue minimum start distance against the verified Betaflight 4.4
  patch bounds: 20–1000 in 4.4.0–4.4.1 and 10–30 in 4.4.2–4.4.3.

- Accept the upstream LED strip `STATUS` profile on all supported Betaflight
  lines. Preserve conditional lookup choices during schema generation, removing
  false validation errors from backups made with LED status mode enabled.

## 0.5.0 — 2026-09-11

- Recognize `feature 3D` and `feature -3D` as valid CLI commands, removing false
  syntax errors and the resulting OSD/serial snippet export blocks.

- Add bundled Betaflight 2025.12 schema and rate support, including named-port
  snippet export. Verify upstream schema inputs, rate functions, and rate defaults
  across releases 2025.12.1–2025.12.5. Custom builds use explicit settings only;
  their omitted defaults remain unknown. Settings with unresolved bounds remain
  excluded from export.

- Inspect Betaflight 2025.12 named serial ports and numeric aliases with correct
  UART identities and Gimbal function labels. Preserve firmware-specific serial
  export tokens.

- Update the roadmap to mark same-version parameter comparison, firmware version
  recognition, and the 2025.12 compatibility investigation complete, with remaining
  compatibility and comparison work listed separately.
- Recognize Betaflight version headers with prerelease or build suffixes, including
  year-based custom builds, instead of displaying “unknown version”. Preserve the
  full version without certifying unsupported firmware defaults.

## 0.4.1 — 2026-09-08

- Fix comparison module resolution on case-insensitive filesystems, which blocked
  Windows and macOS installer builds for 0.4.0.

## 0.4.0 — 2026-09-08

- Add side-by-side parameter comparison for global settings and independently
  selected PID/rate profiles, with changed-value highlighting, filtering, and
  declared/derived/unknown provenance. Expand a source-line link to see the
  original declaration. Missing or invalid values remain unknown; different
  firmware versions are displayed without asserting semantic equivalence.
  CLI collection commands are not included in this comparison.

## 0.3.0 — 2026-09-08

- Add a two-backup angular velocity comparison view with independent rate profiles,
  shared scales for each axis, source/profile labels, hover values in °/s, and
  explicit missing-data and firmware-default provenance messages. Closing a
  selected backup clears its comparison until a replacement is selected.

- Clarify Phase 2's planned multi-backup comparison, explicitly including calculated
  angular velocity overlays and independent profile selection. Prioritize comparison
  before workspace indexing; native platform validation continues alongside it.

## 0.2.2 — 2026-09-08

- Closing a document now forgets every saved path to its imported content, so
  duplicate backups stay closed after restart. Recognized text artifacts also
  close correctly when their identity differs from the native source identifier.

- Mark the Betaflight inspection MVP (Phase 1) complete and revise Phase 2 to
  distinguish delivered restore and packaging from remaining workspace, comparison,
  portable-session, and native integration work. Align project status documentation
  and record Linux/WSL native restore validation, including recovery after file
  permissions are restored, native picking, X11 drag-and-drop, clipboard export
  and snippet saving, plus remaining platform checks.

## 0.2.1 — 2026-09-08

- Collapse the two backup-guidance notices by default: the "For complete Rates
  and PID inspection" note in the sidebar and the "No Betaflight CLI dump all
  marker detected" warning in the Configuration Snapshot. Each keeps its
  headline visible and reveals the detail on click, so the guidance stays
  available without pushing the inspection down the page.

## 0.2.0 — 2026-09-08

- Document version bump criteria, compatibility commitments, and release cadence;
  distinguish feature milestones from major versions and keep patch releases
  unlimited. Align agent guidance and the release checklist with the policy.
- Reopen the backups that were open when FlightLens last closed, and return to
  the document that was in front. Only the file list is saved; every backup is
  read from disk again, so one edited in the meantime is inspected as it now
  stands. A file that has moved or been deleted since is reported by name and
  dropped from the list. Closing a document with `×` keeps it closed.
- Name the flashed target next to the firmware badge in the Configuration
  Snapshot header, as `betaflight 4.3.1 · BETAFPVF4SX1280`, taken from the
  backup's `board_name` line. Backups without one, such as headerless excerpts,
  show the firmware alone.
- Show the craft name and, where the firmware records one, the pilot name in
  the Configuration Snapshot header. Betaflight 4.4 and later declare both as
  settings; 4.2 and 4.3 dumps carry the craft name in their `# name:` header,
  which is read instead. The OSD craft-name preview uses the same name, so it
  is now correct on 4.2 and 4.3 backups as well.

## 0.1.15 — 2026-09-07

- Switch between light and dark modes from the top bar. The choice follows the
  operating system on first run and is remembered afterwards.
- README shows Rates, PID, Filters, Raw and Audit screenshots in both themes,
  captured from the built-in synthetic example.
- Import guidance repeats the complete `dump all` requirement when a backup is
  opened.
- Repository tracks user-visible changes in `CHANGES.md` and outstanding work in
  `TODO.md`, both linked from the README.

## 0.1.14 — 2026-09-07

- Document the complete Betaflight dump requirements for inspection input.

## 0.1.13 — 2026-09-07

- Show complete-backup guidance during import.
- Exclude Cargo output from Vite file watching on Windows.
- Accept Windows line endings when checking generated bindings.

## 0.1.12 — 2026-09-07

- Fix backup inspection and add bundled Betaflight 4.2 compatibility data.

## 0.1.11 — 2026-09-07

- Say why no firmware defaults were read back.

## 0.1.10 — 2026-09-07

- Read omitted rate values from certified firmware defaults.

## 0.1.9 — 2026-09-06

- Show all configuration profiles.

## 0.1.8 — 2026-09-06

- Choose populated profiles for inspection.

## 0.1.7 — 2026-09-06

- Fix Betaflight patch release compatibility.

## 0.1.6 — 2026-09-06

- Release FlightLens 0.1.6.

## 0.1.5 — 2026-09-06

- Support vendor-suffixed Betaflight versions.

## 0.1.4 — 2026-09-06

- Add the macOS release workflow and publish the roadmap.

## 0.1.3 — 2026-09-06

- Fix Windows installer artifact paths.

## 0.1.2 — 2026-09-06

- Include the Windows ICO in the desktop bundle.

## 0.1.1 — 2026-09-06

- Add the Windows icon resource.

## 0.1.0 — 2026-09-06

- First packaged build: Betaflight backup ingestion, rates, PID, filters,
  ports, modes, OSD, raw source and audit views, snippet export, and the
  Windows installer workflow.
