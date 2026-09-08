# Changes

User-visible changes to FlightLens, newest first. Each released section matches
the `v<version>` tag whose build produced the Windows and macOS installers.
Work that is committed but not yet released sits under `## Unreleased` and is
renamed to the version when the release is cut. Purely internal work
(formatting, comments, test-only refactors) is left out.

## Unreleased

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
