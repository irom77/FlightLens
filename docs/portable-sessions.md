# Portable sessions

Phase 2 item 6 is implemented in FlightLens 0.9.0 within the boundary below;
packaged Windows/macOS validation remains pending. Native Save Session As/Open Session controls save
file-backed CLI and recognized firmware text references and restore matching documents, workspace folder,
active document, comparison/profile selections, tab and theme.
Opening adds to the current workspace. A native confirmation lists references
before reading their contents. Changed or missing files are skipped with messages;
reopen a saved session to retry backup references. Save As creates a new file and refuses to
overwrite existing files, including backups. Pasted documents and Blackbox imports
must be closed before saving. File-backed recognized iNav/ArduPilot text backups
can be saved and restored as recognition-only artifacts, with the saved active
selection. They do not become eligible for CLI comparison or profile inspection.
Save uses the hash recorded at import, without rereading the backup; a later
change is detected when reopening. Closing a recognized artifact removes its
file association. Blackbox imports currently read only a header prefix, so they
have no full-content hash suitable for a portable reference.

Workspace locations are restored after the native confirmation lists the folder.
The explorer starts the existing bounded recursive metadata scan; backup contents
are read only when opened. A missing, inaccessible or incompatible folder leaves
the current explorer intact and reports a warning; reopen the session to retry.
Save As retains that unavailable folder with its path rebased to the new session
location, even while the explorer shows a previously opened folder. Successfully
choosing a workspace folder explicitly replaces the retained location; cancelling,
a failed choice, or refreshing the existing explorer leaves recovery intact.
An incompatible foreign workspace path blocks saving until a folder is chosen.
A manifest without a workspace also preserves the current explorer.

Comparison order and baseline are restored only when all selected backups reopen
as distinct CLI documents with matching hashes. Otherwise the inspector opens
and a warning explains why the comparison was not restored.
Rate/PID selections are shared per backup between inspector and comparison;
switching views keeps them. Saved profile numbers are checked against reopened
documents. An unavailable number produces a message naming the fallback profile.
Saving while comparing requires two or three available selected backups; saving
from the inspector records no active comparison unless an unavailable saved
comparison is being retained. Search/filter controls inside
comparison tables are not session layout preferences.

Failed backup references from the last confirmed session are retained in native
memory during this app run and included in Save As, even when no backups loaded.
Paths are rebased to the new manifest directory; hashes and profile numbers stay
unchanged. Retained entries count toward the 32-document limit. A matching open
path/hash is saved once using current profiles; conflicting hashes or incompatible
foreign paths block saving before a file is created. Reopening a saved session
retries these references. Cancellation and invalid manifests retain the recovery
list; opening another confirmed session replaces it. Unavailable active and
comparison selections also survive Save As, retaining their exact backup
references, comparison order and baseline. Those references may include a loaded
backup subsequently closed in the inspector. Uncheck “Keep unavailable
active/comparison selections when saving” to use the current view instead;
failed backup references remain retained independently. A new available comparison
takes precedence over the saved unavailable comparison. Reopening a confirmed
session resets the checkbox.

Relink session backups retries the last confirmed manifest and opens a native file
picker for each reference that cannot be read or fails its saved hash check.
The picker title identifies the saved reference. Cancelling a picker skips that
reference and continues; a hash mismatch rejects the replacement and leaves the
original reference unresolved. Accepted replacements restore saved profiles,
active selection, and comparison order/baseline through the normal restore flow.
Use Save As to record replacement paths, then open the new manifest to make it
the retry target. The original manifest and backups are unchanged. Relinking
also repeats reference confirmation and restores saved preferences. Workspace
folders are replaced using the existing Choose folder control.

Retry session rereads the last confirmed manifest from disk without opening a
file picker. It repeats validation, reference confirmation and hash checks, then
restores that manifest's saved selections and retries its workspace scan. Save As
does not change the retry target: open the new session to make it the target.
Unsaved selection changes may be replaced on successful retry. Cancelled and
invalid opens leave the previous retry target and recovery state intact. If the
manifest itself moved, use Open session to select its new location.
The existing automatic `session.json` restoration is unchanged.

## Version 1 contract

A `.flightlens` file is UTF-8 JSON with `format: "flightlens"` and `version: 1`.
It contains a nullable workspace path, at most 32 document references, a nullable
active document index, a nullable comparison, and tab/theme preferences.
Each document reference contains `path`, the lowercase 64-character `sha256` of
its source bytes, `rateProfile`, and `pidProfile`. Profiles are unsigned byte
indices; their availability must be checked against the reopened document.
Comparison `documents` contains two or three distinct document indices in graph
order. Its nullable `baseline` is also a document index and must be one of them.
A missing baseline preserves the UI's unselected baseline state.

The core rejects unsupported versions, unknown fields, invalid indices, duplicate
path strings, malformed hashes, invalid preferences, and manifests over 256 KiB.
Errors exclude manifest contents. Saving applies the same validation as reading.
The format includes no backup contents. Paths can contain private directory names;
portable means movable with its referenced files, not automatically redacted.

## Paths and recovery

Paths beneath the manifest's containing directory are saved relatively; other
paths remain absolute. Relative paths resolve against that directory, never the
application working directory. Moving the manifest together with its backup tree
therefore preserves references. Native Windows separators are saved as `/`.
Foreign absolute paths require relinking on the destination operating system.
The path helpers do not open files, canonicalize paths, or grant filesystem access.

## Item 6 completion boundary

Item 6 is complete when a user can explicitly save and reopen a **reference-only**
workspace for supported text backups, recover unavailable locations without
silently substituting changed contents, and understand what cannot be saved.
The local implementation meets these criteria:

| Required behavior | Implementation and evidence |
| --- | --- |
| Explicit Save As and Open | Native pickers; create-new writes refuse overwrites; browser save/open tests |
| Versioned, bounded metadata | Version 1 codec; 256 KiB / 32-reference bounds; strict schema and compatibility tests |
| Workspace and document locations | Relative paths beneath the manifest folder, absolute paths elsewhere; moved-tree/path tests and workspace restore browser checks |
| Saved selections | Active backup, inspector tab, theme, per-backup rate/PID profiles, two/three-backup comparison order and baseline; native index-mapping and browser round-trip tests |
| Missing or changed backups | Per-reference errors; original hashes retained; partial restores, Retry, and native hash-checked replacement pickers |
| Recovery survives Save As | Failed references, unavailable workspace and active/comparison selections retained and rebased; native retention/conflict/limit tests |
| Safe cancellation and invalid manifests | No restore before validation/confirmation; previous recovery target remains; browser cancellation and invalid-open tests |
| Clear input limits | File-backed CLI and recognized firmware text accepted; pasted sources and header-only Blackbox imports rejected before the save picker; native source eligibility tests |
| Offline, read-only operation | No embedded backup contents, backup writes or network calls; native references are backend-owned |

“Layout preferences” means active document, inspector tab and theme. Window
geometry, expanded panels, comparison search/filters and explorer pagination
are transient and are not serialized. Opening is additive rather than replacing
every currently open document. Duplicate contents share document identity;
comparisons require distinct supported configurations.

Pasted backups must be saved separately and opened as files before inclusion.
The app does not silently omit them or write their contents as part of session
saving. Embedded backups are a separate future format decision. Full-file
Blackbox hashing/references belong with Phase 3 telemetry work; the current
header-only import cannot provide the required content identity.

The existing native confirmation and replacement pickers satisfy location review
for this item. Finer per-location permission controls are optional future work.
Changed contents are inspected by opening the file separately; relinking only
accepts the saved hash. Save As and opening the new manifest establish a new
session/retry target.

OS associations and delivery to an already-running app are deferred in TODO (former Phase 2 item 7).
Packaged Windows/macOS dialogs, drive disconnect/reconnect, cancellation and
cross-platform path smoke checks are deferred in TODO (former Phase 2 item 8). Local completion does
not claim those platform checks or release publication have happened.

## Validation

Core tests cover round-trip metadata, unsupported versions, strict field handling,
size limits, invalid document/comparison references, malformed hashes, moved
relative trees, and foreign Windows paths on Unix. A browser IPC test covers control wiring, partial restore messages, saved UI
metadata, restored workspace visibility, cancellation and invalid manifests.
Additional tests cover comparison ordering/baseline mapping, missing or duplicate
comparison sources, unavailable profile messages, and browser save/open round trips
for three-backup graphs and profile selections. Native recovery tests cover moved
Save As paths, preserved hashes/profiles, deduplication, conflicting hashes, and
the combined document limit, and missing workspace rebasing/replacement, and selection index remapping after
merging open and retained references. Relink checks cover matching, changed, and
missing replacement files; browser checks cover the relink command wiring.
Recognized-text tests cover imported file associations, missing files at save
time, active-selection/hash serialization, and removal on close.
Native compilation is checked on
Linux; packaged Windows/macOS dialog behavior still needs validation.

### Completion checkpoint — 2026-09-13

- `./test.sh` passed after the final runtime change: core tests, generated
  bindings, TypeScript, 80 frontend tests, 24 browser tests, and the 94-file
  corpus (zero failures; three existing parse diagnostics).
- `pnpm screenshots:check` passed both themes after the final runtime change.
- `cargo test -p flightlens portable::tests` passed all eight tests at boundary
  closeout, including the added pasted CLI/iNav/ArduPilot rejection check.
- Rust formatting and `git diff --check` passed at closeout.
- No commit, release, or packaged Windows/macOS validation is implied.
