# Document storage and IPC assessment

Measured 2026-09-15 for review C2. The duplication is real; reducing it requires
preserving comparison semantics as well as paginating the Raw view.

## Reproduce offline

Only the repository's built-in synthetic fixture and generated comments/settings
are used. The probe accepts a named scenario, not a backup path. No user input is
read, logged or transmitted. No new dependencies are needed.

```sh
cargo build --release -p flightlens-core --bin document_cost
node --expose-gc tools/measure_document_cost.mjs
```

The Rust binary writes a synthetic Artifact or ArtifactView to stdout and aggregate metrics to
stderr; the Node wrapper captures the payload and prints only aggregate results.
Run the wrapper when collecting measurements. The four size scenarios pad the
fixture with 256-byte comments; settings pads it with repeated motor_poles
declarations. Inputs respect both parser limits: 16 MiB and 100,000 lines.
These are controlled shape/size probes, not a representative user corpus.

## Observations

One local Linux run, release Rust 1.98.1, Node 24.15.0. Times vary by machine and
run; these are observations, not thresholds or medians.

| Synthetic input | Lines | Input bytes | Artifact JSON bytes | Syntax JSON bytes | Retained parsed JS heap | JSON.parse |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Built-in fixture | 55 | 1,254 | 21,264 | 6,759 | 43,168 B | 0.16 ms |
| 64 KiB comments | 306 | 65,510 | 104,985 | 90,480 | 141,040 B | 0.27 ms |
| 1 MiB comments | 4,146 | 1,048,550 | 1,398,482 | 1,383,977 | 1,584,752 B | 2.28 ms |
| 16 MiB comments | 65,586 | 16,777,190 | 22,335,176 | 22,320,671 | 24,686,112 B | 40.06 ms |
| 1 MiB settings | 49,927 | 1,048,566 | 6,587,826 | 6,573,318 | 9,622,256 B | 15.95 ms |

For the near-16 MiB input, Rust parsing took 19.56 ms, a full document clone
7.11 ms, and JSON serialization 12.89 ms. The settings case took 18.94, 3.61,
and 6.25 ms respectively. Clone timing excludes dropping the clone.

Parsed JS heap is the Node/V8 heap delta after explicit garbage collection,
with the JSON string already retained before the baseline. It excludes the
payload string, native buffers, Rust objects, IPC transport copies, DOM, React,
and analysis results. It is not total application memory or a measurement of
Windows WebView2/macOS WKWebView. The probe does not measure IPC latency,
renderer responsiveness, peak allocation, or a full 32-document session.

## Compact source evidence (2026-09-16)

`ConfigDocument::source_evidence()` now produces a typed `SourceEvidence` with
line count, dump-all marker status and ordered comparison syntax. It removes
comments, blanks and settings no longer referenced by any final parameter.
Unknown-scope settings remain separate declarations, matching the parser.
Collections, defaults, malformed commands, feature/port/mode declarations and
other command kinds retain their original order, raw text, line numbers and
offsets. This is inspection evidence, never an exportable backup.

The synthetic probe now reports `evidenceBytes` and `evidenceLines` alongside
the full payload measurements. A local run produced:

| Synthetic input | Full syntax JSON bytes | Evidence JSON bytes | Evidence lines |
| --- | ---: | ---: | ---: |
| Built-in fixture | 6,759 | 6,412 | 52 |
| 64 KiB comments | 90,480 | 6,413 | 52 |
| 1 MiB comments | 1,383,977 | 6,414 | 52 |
| 16 MiB comments | 22,320,671 | 6,415 | 52 |
| 1 MiB settings | 6,573,318 | 6,426 | 52 |

These are source-evidence bytes only, not a complete new document DTO or a
runtime memory saving. Full artifacts still cross IPC. Large collections,
malformed commands and unknown-scope settings can still produce large evidence;
this is not a constant-size bound. Diagnostics and other document fields remain
outside these evidence measurements.

Five tests cover marker recognition, retained parameter sources across scopes
(including invalid values), ordered replay on verified and unknown firmware,
unknown-scope declarations, and repeated-comment/setting reduction. The frontend now defines `DocumentView` without a full `syntax` field, and all
comparison functions and their components accept either this view or a full
document. Shared accessors retrieve ordered evidence and source declarations.
Imports and the Raw viewer still use full documents.

`src/documentView.test.ts` runs the Rust synthetic `comparison_evidence` binary
and checks all 196 pairs of 14 documents against the full-document results:
collections, RX ranges/failsafe, adjustment ranges, VTX tables/activations,
features, ports, modes and parameters. It also checks source references and
three-document parameter comparisons. Cases include changed values, resets,
malformed declarations, recovery after invalidation, multiple profiles and
unknown firmware. Mixed full/compact pairs are covered during migration.

The frontend test suite now needs Cargo on PATH (or the CARGO environment
variable); `test.sh` already supplies it and CI installs Rust. Node type
definitions are a development-only dependency for this cross-language test.
The test binary accepts no user input and uses only built-in synthetic text.

The runtime now returns the generated view from every import/session path and
pages raw syntax separately, as described below. These tests prove the comparison
boundary; native and browser tests cover storage and navigation.

## Completed runtime boundary (2026-09-16)

- AppState moves the original ConfigDocument into an Arc. It neither clones the
  full syntax at import nor during analysis/export/feedback/session lookups.
- Paste, file open, workspace open and portable-session restore return the
  generated ArtifactView. Startup session restore supplies source references,
  which are opened through the same compact file-open path. Recognized artifacts
  keep their original shape.
- The renderer stores only DocumentView inspection fields and compact source
  evidence. Inspector uses its line count and dump-all marker directly.
- Raw requests at most 500 syntax lines by content-hash document ID and offset.
  IDs identify immutable source content: reimporting identical content may
  replace metadata, but its raw lines are identical. Missing/closed IDs error;
  offsets past EOF return an empty page; invalid counts are rejected. Snapshot
  handles allow requests already acquired before close to finish safely.
- Raw retains only its current page, clears it on page/document change, ignores
  stale replies/errors, and offers retry on failure. Source jumps scroll after
  the requested lines arrive. Unmounting Raw discards its page/request result.
- The 128 MiB repository limit still counts raw input text, not heap. Documents
  with substantial comparison evidence/diagnostics can still have large views;
  there is no constant-size DTO or total application-memory guarantee.

## Full versus compact import measurements (2026-09-16)

One local run with the same release Rust 1.98.1 and Node 24.15.0. The wrapper
now runs both formats for each scenario. These are whole artifact payloads,
including metadata, parameters, diagnostics and comparison evidence.

| Synthetic input | Full JSON bytes | Compact JSON bytes | Full parsed JS heap | Compact parsed JS heap | Full / compact JSON.parse |
| --- | ---: | ---: | ---: | ---: | ---: |
| Built-in fixture | 21,264 | 20,925 | 43,168 B | 46,496 B | 0.290 / 0.173 ms |
| 64 KiB comments | 104,985 | 20,926 | 141,456 B | 46,480 B | 0.357 / 0.181 ms |
| 1 MiB comments | 1,398,482 | 20,927 | 1,592,040 B | 46,936 B | 2.417 / 0.163 ms |
| 16 MiB comments | 22,335,176 | 20,928 | 24,686,064 B | 46,320 B | 50.362 / 0.172 ms |
| 1 MiB settings | 6,587,826 | 20,942 | 9,621,864 B | 46,320 B | 21.102 / 0.196 ms |

Compact serialization includes projecting the view: 22.660 ms for the 16 MiB
case versus 14.923 ms for full serialization in this run. Scanning source
markers/evidence still costs backend work. Clone timing in the probe is a
baseline comparison only; runtime no longer makes that full clone. Small-input
heap deltas are noisy and the smallest case shows no heap reduction. All prior
measurement limitations apply, particularly that Node heap is not desktop heap.
Packaged Windows/macOS memory and interaction latency remain a separate follow-up.

## Syntax consumers that a smaller DTO must preserve

The review's claim that only Raw reads syntax is incorrect:

- Inspector counts lines and detects the dump-all marker.
- Parameter, port, mode and feature comparisons retrieve source declarations.
- Collection comparison groups and compares raw collection declarations.
- RX ranges, RX failsafe, adjustment ranges, VTX activations and VTX tables
  replay syntax in order, including defaults, resets and malformed declarations
  that invalidate earlier values.

Dropping syntax or retaining only successfully parsed commands would silently
change Unknown/Matching/Different outcomes. Fetching raw pages only when Raw
opens cannot support these consumers.

## Verification and completion

C2's runtime implementation is complete. Native tests verify that import retains
the original syntax allocation, serializes only the view, preserves exact raw
Unicode/CRLF/final-line content across pages, bounds page sizes/offsets, and
handles close/replacement snapshot lifetimes. Browser coverage uses actual Rust
compact DTOs for imports, restored comparisons and source jumps. A delayed IPC
test verifies portable-session Raw restore, page races, retry, same-ID reload,
and closing with a request outstanding. Cross-language tests verify 196 pairs
and three-document comparisons against full documents.

Packaged OS performance measurements are still needed before claiming a measured
end-user latency or total-memory improvement. They do not block removing the
full source duplication from the renderer.
