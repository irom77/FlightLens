# Workspace explorer

Choose **Choose workspace folder…** to discover backup files recursively. The
explorer keeps one folder index, separately from the open-document repository.
Search matches file names and relative folder paths, case-insensitively; results
are paginated in groups of 50. Selecting a result reads and inspects that file
using the same immutable import pipeline as Open backups. Indexing alone never
opens a document, reads backup contents or grants firmware support.

The index stores only paths, identifiers, file sizes and modification times.
Candidate extensions are `.txt`, `.diff`, `.dump`, `.param`, `.parm`, `.bbl` and
`.bfl` (case-insensitive). Unsupported formats still receive the existing
recognition/capability result when opened. There is no content or firmware search.
Folder selection and metadata last for this app run; portable workspace/session
persistence belongs to Phase 2 item 6. Opened files use existing session restore.

## Work limits and partial results

One background worker advances discovery in batches of at most 256 traversal
steps or 50 ms between filesystem calls. The explorer requests batches while
scanning and otherwise requests metadata only when search/page selection changes.
It stores at most 10,000 backup entries, visits at most 100,000 directory entries,
and holds at most 64 directory iterators. A limit produces an explicit partial
result; select a smaller folder to discover the remainder. Final results sort by
relative path; results may move between pages while discovery is in progress.

Symbolic links, inaccessible entries and deeper folders are skipped and counted.
Cancel keeps metadata found so far and closes traversal iterators when the worker
observes cancellation. Filesystem calls can block on a disconnected or stalled
volume; cancellation cannot interrupt an individual OS call. It takes effect
between calls, without starting another index worker.

## Changed files and removable drives

The index is a discovery snapshot, not a watcher. **Refresh workspace** replaces
it with a fresh scan. A failed refresh retains the existing metadata and reports
that the folder is unavailable so the user can reconnect and retry. Opening a
missing entry reports an error; it does not discard already open snapshots.
Opening reads the current file, not cached contents. Refresh invalidates prior
entry identifiers. Entries that now resolve outside the selected folder are
rejected. No backup is changed, and metadata is not sent to a service.

## Validation

Rust tests cover nested discovery, filename search, pagination, binary candidates
without content parsing, missing entries, stale identifiers, cancellation,
traversal/depth limits, disconnect/reconnect and Unix link handling. Browser tests
cover the explorer controls, partial scans, opening on demand, late search
responses, missing sources and refresh/retry. Desktop compilation is checked on
Linux; packaged Windows/macOS folder picking and physical removable-media smoke
checks remain part of Phase 2 item 8.
