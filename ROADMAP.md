# FlightLens roadmap

FlightLens is developed as an offline-first inspection tool for flight controller backups. Each phase keeps imported files read-only, preserves source provenance, and adds support behind explicit capability and compatibility checks.

## Phase 1 — Betaflight inspection MVP

The first release provides the complete Betaflight configuration workflow:

- Paste, file, and drag-and-drop ingestion into immutable snapshots.
- Lossless CLI parsing with source-line diagnostics and explicit unknown values.
- Versioned Betaflight 4.3, 4.4, and 4.5 compatibility data.
- Rates, PID, filters, ports, modes, OSD, raw source, and audit views.
- Semantic audits with insufficient-data results instead of unsafe assumptions.
- Dependency-aware snippet export that reparses and validates before copy/save.

## Phase 2 — Workspace and comparison

Phase 2 expands the desktop workflow around multiple backups:

- Recursive workspace indexing with pagination, search, cancellation, and removable-media handling.
- Native file associations and searchable explorer navigation.
- Two- and three-configuration semantic comparison with profile mapping and provenance badges.
- Explicitly saved `.flightlens` sessions and layout preferences.
- Packaged Windows and macOS releases with native integration smoke tests.

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
