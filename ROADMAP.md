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

## Phase 2 — Workspace and comparison — in progress

Phase 2 builds on foundations already shipped:

- [x] Multiple open documents with content-hash identity. Closing a document
  removes every saved path associated with its imported content (fixed in 0.2.2).
- [x] Automatic reopening of file-backed documents, with active-document and tab
  preferences; persisted light/dark theme selection.
- [x] Windows and macOS installer packaging and tag-triggered release workflows.

Comparison is the next feature priority; recursive indexing is not a prerequisite.
Native installer validation continues alongside feature development.

Remaining work, in implementation order:

1. [x] Add a comparison view where users select two imported backups, including
   files opened together through the multi-file picker. Select each backup's rate
   profile independently and overlay calculated roll, pitch, and yaw angular
   velocity against stick input on shared scales (°/s). Label each source and
   profile, show values at the same stick position, and explicitly report missing
   or unsupported curves. These are configuration-derived responses, not measured
   flight motion; Blackbox angular velocity remains in Phase 3.
2. [ ] Extend the two-backup view with semantic parameter comparison, independent
   PID/rate profile mapping, source-line links, and declared/derived/unknown
   provenance badges. Distinguish unknown values from changes; compare versions
   only where bundled compatibility data establishes equivalent semantics.
   Same-version parameter comparison, independent profiles, source declarations,
   and provenance are implemented; cross-version equivalence mappings remain.
3. [ ] Extend comparison to three selected configurations, including angular
   velocity overlays. Use an explicitly selected baseline for semantic differences,
   distinguishing changes on either side and conflicting changes. Comparison
   remains read-only; it does not merge or apply configurations.
4. [ ] Add recursive workspace indexing and searchable explorer navigation, with
   pagination, cancellation, bounded background work, and removable-media
   handling. Index metadata without keeping every backup open in memory.
5. [ ] Add explicitly saved/opened `.flightlens` sessions for workspace locations,
   document references, comparison/profile selections, and layout preferences.
   Define format versioning, compatibility, relative-path resolution, and recovery
   for moved or changed files. Automatic `session.json` restore is already present;
   portable sessions are not.
6. [ ] Add native file associations and OS open-file routing, including opening
   files in an already-running app. Keep firmware and telemetry capability checks
   explicit when a recognized format is not yet supported.
7. [ ] Validate the desktop workflow on Windows and macOS: compile the desktop
   crate before release publication and record native smoke checks for picking,
   dropping, OS file opening, restore, clipboard, and snippet saving. Existing
   installer builds and browser tests do not replace native integration checks.
   Linux/WSL restore checks cover duplicate, changed, missing, and unreadable
   sources; Windows/macOS restore and disconnected sources remain to be checked.

Signing and notarization remain a separate distribution decision tracked in
[TODO.md](TODO.md); initial installer packaging is already delivered. Phase 2 is
complete when the remaining workflow is implemented and verified, with saved-data
compatibility and platform limitations documented.

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
