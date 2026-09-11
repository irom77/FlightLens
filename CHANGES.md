# Changes

User-visible changes to FlightLens, newest first. Each released section matches
the `v<version>` tag whose build produced the Windows and macOS installers.
Work that is committed but not yet released sits under `## Unreleased` and is
renamed to the version when the release is cut. Purely internal work
(formatting, comments, test-only refactors) is left out.

## Unreleased

- Send feedback from inside the app: a subject, a description, and optionally
  the open configuration. FlightLens opens a prefilled GitHub issue in your
  browser and copies the configuration to your clipboard for you to paste; it
  transmits nothing itself and needs no account or server. Names and radio link
  identities are replaced before copying, and everything that leaves the app is
  shown for review first.

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
