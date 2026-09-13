# FlightLens roadmap

FlightLens is developed as an offline-first inspection tool for flight controller backups. Each phase keeps imported files read-only, preserves source provenance, and adds support behind explicit capability and compatibility checks.

## Phase 1 — Betaflight inspection MVP — completed

The inspection MVP is implemented as of 0.2.1:

- [x] Paste, file, and drag-and-drop ingestion into immutable snapshots.
- [x] Lossless CLI parsing with source-line diagnostics and explicit unknown values.
- [x] Versioned Betaflight 4.2, 4.3, 4.4, and 4.5 compatibility data.
- [x] Rates, PID, filters, ports, modes, OSD, raw source, and audit views.
- [x] Semantic audits with insufficient-data results instead of unsafe assumptions.
- [x] Dependency-aware snippet export that reparses and validates before copy/save.

Completion covers this feature scope, not every possible Betaflight command or
native-platform validation. Unsupported semantics and missing dependencies remain
explicitly unavailable. Desktop smoke checks and known defects remain tracked in
[TODO.md](TODO.md). Phase completion does not imply 1.0 readiness or choose a
version bump; follow the [release policy](docs/releases/README.md).

### Betaflight compatibility follow-up — in progress

- [x] Recognize prerelease and build suffixes in firmware headers, including
  year-based custom builds. Shipped in 0.5.0; regression tests and the full
  check script passed against the Windows DUMP_ALL corpus. Version recognition does not certify firmware semantics or defaults.
- [x] Investigate upstream 2025.12 compatibility and classify the local corpus
  diagnostics. Record source pins, serial syntax changes, and schema/rate
  verification requirements in [the investigation](docs/betaflight-2025.12-compatibility.md).
- [x] Support named serial ports and preserve firmware-appropriate export syntax.
  Full checks pass against the now 14-file Windows corpus; all 15 original serial
  syntax errors are resolved.
- [x] Add a verified, bundled 2025.12 schema and rate reference coverage. Schema
  inputs, rate vectors, and reset defaults are verified across 2025.12.1–2025.12.5;
  custom-build defaults remain uncertified. Supported-version guidance is updated.
  Twelve settings with unresolved bounds remain excluded from export (see TODO).
- [x] Accept numeric feature names such as `3D`, including disable commands.
  Removes all 14 feature syntax errors in the Windows corpus and their false
  OSD/serial export blocks.
- [x] Preserve conditional LED lookup choices, including `STATUS`, across all
  five bundled firmware lines. Removes 14 false schema diagnostics.
- [x] Correct GPS rescue validation with verified patch-specific bounds for
  Betaflight 4.4.0–4.4.3, including vendor suffixes. The remaining corpus
  diagnostic is an OSD units value outside the upstream choices.
  LED and GPS fixes are completed locally and await release.

## Phase 2 — Workspace and comparison — in progress

Phase 2 builds on foundations already shipped:

- [x] Multiple open documents with content-hash identity. Closing a document
  removes every saved path associated with its imported content (fixed in 0.2.2).
- [x] Automatic reopening of file-backed documents, with active-document and tab
  preferences; persisted light/dark theme selection.
- [x] Windows and macOS installer packaging and tag-triggered release workflows.

Phase 2.5 implementation is complete. Resume Phase 2 at item 4; recursive indexing is not
a prerequisite. Betaflight compatibility follow-up and native installer
validation remain tracked alongside feature development.

Comparison progress and remaining work, in implementation order:

1. [x] Add a comparison view where users select two imported backups, including
   files opened together through the multi-file picker. Select each backup's rate
   profile independently and overlay calculated roll, pitch, and yaw angular
   velocity against stick input on shared scales (°/s). Label each source and
   profile, show values at the same stick position, and explicitly report missing
   or unsupported curves. These are configuration-derived responses, not measured
   flight motion; Blackbox angular velocity remains in Phase 3.
2. [x] Add same-version parameter comparison for global settings and independently
   selected PID/rate profiles, source-line links, and declared/derived/unknown
   provenance. Distinguish unknown values from changes. Shipped in 0.4.0, with
   the Windows/macOS module-resolution build fix in 0.4.1.
3. [x] Deliver a bounded first set of certified cross-version parameter mappings
   and CLI collection comparisons. **Completed in 0.7.0.** The
   completion boundary is the following implemented and tested scope:
   - Eleven global integer mappings across official Betaflight 4.5.0–4.5.5 and
     2025.12.1–2025.12.5: `motor_poles`, `vbat_max_cell_voltage`,
     `vbat_min_cell_voltage`, `vbat_warning_cell_voltage`, `bat_capacity`,
     `force_battery_cell_count`, `vbat_divider`, `vbat_multiplier`, `ibata_offset`,
     `ibatv_scale`, and `ibatv_offset`. Each requires verified bounds, exact
     release/family/pack gates and bundled upstream evidence. See
     [parameter equivalence](docs/parameter-equivalence.md).
   - Same-version feature, serial-port and mode-assignment declaration comparison.
   - Source-text comparison for all five recognized collections: `vtxtable`,
     `vtx`, `rxfail`, `rxrange`, and `adjrange`. Certified same-version comparison
     covers explicit VTX tables, rxrange endpoints, rxfail channel declarations
     and adjustment-range declarations for the verified official releases;
     VTX activation comparison is limited to official 4.5.0–4.5.5. See
     [VTX](docs/vtx-comparison.md), [rxfail](docs/rxfail-comparison.md), and
     [adjustment-range](docs/adjrange-comparison.md) evidence for exact gates.
   - Comparisons preserve source lines and provenance, distinguish missing or
     invalid values from changes, and report unsupported equivalence as not
     comparable. Collection comparisons respect reviewed command ordering,
     resets and normalization. Backups remain read-only and comparison offline.
   - Bounds, unsupported identities, unknown values and parser-to-renderer
     behavior have regression coverage. `./test.sh` and `pnpm screenshots:check`
     passed at the implementation checkpoint on 2026-09-12. Packaged native
     validation remains required under item 8.
   Additional mappings, firmware coverage, cross-version collection equivalence,
   collection diagnostics/export and runtime equivalence are follow-up work,
   not prerequisites for this item or Phase 2. The dump-processing guide is a
   separate documentation task. Do not reopen this item for coverage expansion;
   defects in the delivered scope still require fixes.
4. [ ] Extend comparison to three selected configurations, including angular
   velocity overlays. Use an explicitly selected baseline for semantic differences,
   distinguishing changes on either side and conflicting changes. Comparison
   remains read-only; it does not merge or apply configurations.
5. [ ] Add recursive workspace indexing and searchable explorer navigation, with
   pagination, cancellation, bounded background work, and removable-media
   handling. Index metadata without keeping every backup open in memory.
6. [ ] Add explicitly saved/opened `.flightlens` sessions for workspace locations,
   document references, comparison/profile selections, and layout preferences.
   Define format versioning, compatibility, relative-path resolution, and recovery
   for moved or changed files. Automatic `session.json` restore is already present;
   portable sessions are not.
7. [ ] Add native file associations and OS open-file routing, including opening
   files in an already-running app. Keep firmware and telemetry capability checks
   explicit when a recognized format is not yet supported.
8. [ ] Validate the desktop workflow on Windows and macOS: compile the desktop
   crate before release publication and record native smoke checks for picking,
   dropping, OS file opening, restore, clipboard, and snippet saving. Existing
   installer builds and browser tests do not replace native integration checks.
   Linux/WSL restore checks cover duplicate, changed, missing, and unreadable
   sources; Windows/macOS restore and disconnected sources remain to be checked.

Signing and notarization remain a separate distribution decision tracked in
[TODO.md](TODO.md); initial installer packaging is already delivered. Phase 2 is
complete when items 1–8 are implemented and verified, with saved-data
compatibility and platform limitations documented. Item 3 has a fixed coverage
boundary above; further comparison research does not block items 4–8. The next
Phase 2 feature unit is item 4, baseline-based three-backup comparison, after
Phase 2.5 is complete.

## Phase 2.5 — Feedback and inspection improvements — implemented

Phase 2.5 collects user-facing improvements that do not depend on the Phase 3
telemetry pipeline. Both items are implemented. Resume Phase 2 at item 4. Outstanding
packaged Windows/macOS checks remain tracked in TODO and Phase 2 item 8.

1. [x] Add a feedback button that collects a subject, a body, and optionally the
   current configuration. The app opens a prefilled GitHub issue in the browser
   and places a redacted configuration on the clipboard for the reporter to
   paste, so FlightLens never transmits backup contents itself and needs no
   server, credentials, or captcha. The reporter files the issue and maintainers
   triage it on GitHub. Require complete fields before submitting, make the
   configuration opt-in, and show the redacted payload before it leaves the app.
   The dialog states that submission requires a GitHub account before the form,
   with a “Continue to GitHub” button for the browser hand-off.
   Review fixes complete locally: submission requires a matching preview and
   attachment size checks include the report text. Opening the browser and reaching the clipboard remain to be
   checked in packaged Windows and macOS builds (see TODO).
2. [x] Add a Throttle Curve Preview to the Rate Profile tab, alongside the
   existing Rates Preview, with Throttle Limit, Throttle MID, and Throttle EXPO
   controls, so pilots can visualize throttle response next to stick rates.
   The preview is read-only and derived from imported `thr_mid`, `thr_expo`,
   `throttle_limit_type`, and `throttle_limit_percent` values; versions without a
   verified throttle mapping report the preview as unavailable.
   Source review is documented in [throttle preview evidence](docs/throttle-curve-preview.md).
   Calculation checkpoint complete: Rust legacy lookup and limit calculation,
   exact release/schema gates, explicit profile inputs, and regression tests.
   Upstream C differential validation is complete: 28,512 vectors across all
   nine supported releases. Inspector integration, read-only values with source
   links, percentage axes, and profile/unavailable-state browser coverage complete.
   The initial implementation uses the verified legacy mapping; 2025.12's
   different `thr_hover`-dependent curve is deferred and must report unavailable.
   Completed with upstream differential fixtures, profile/provenance and
   unavailable-state coverage, and a read-only preview with percentage axes.
   Additional firmware coverage does not block resuming Phase 2 item 4.

## Phase 3 — Blackbox telemetry and diagnostics

Phase 3 adds the telemetry pipeline before expanding configuration adapters:

- Native Rust Blackbox indexing and header-defined decoding.
- Session manifests, calibrated time-series chunks, timestamp gaps, and corruption recovery.
- Viewport-sized plots, Log A/B alignment, spectra, response metrics, saturation, and voltage-sag summaries.
- Differential fixtures against trusted decoder output and bounded background workers.
- Redacted, user-reviewed diagnostic payloads with optional local or cloud providers.

Cloud diagnostics remain opt-in. No network request, credential storage, configuration mutation, or tuning recommendation happens automatically.

## Phase 4 — iNAV and ArduPilot adapters

Phase 4 broadens configuration support while keeping each firmware family independent:

- iNAV CLI schemas, profile semantics, navigation, fixed-wing mixer, waypoint availability, exports, and audits.
- ArduPilot `.param`/`.parm` parsing with versioned vehicle metadata, parameter trees, and native delta exports.
- Explicit handling for missing vehicle/version context and unsupported mission artifacts.
- Independent fixture coverage so Betaflight behavior remains stable.

Cross-firmware migration is outside the scope of these phases. A field is compared across firmware only when a compatibility mapping proves equivalent semantics.

## Later work

Future releases may add DataFlash log support, signed compatibility-pack updates, richer platform packaging, and additional audit coverage. These remain subject to fixture-backed validation and the offline privacy boundary.
