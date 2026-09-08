# TODO

Outstanding work on FlightLens. Items move out of this file when they ship: a
user-visible one becomes a `CHANGES.md` entry, an internal one just disappears
with the commit. Long-range direction lives in [ROADMAP.md](ROADMAP.md); this
file holds only what is actionable now.

## Next

- Two copies of the same backup, opened from different paths, are one document
  (the id is the content hash) but two entries in the saved session. Closing
  that document drops only the path it was last opened from, so the other copy
  reopens on the next run. Either key the session by document id or forget
  every path that resolves to the closed document.
- Compile `src-tauri` somewhere. Rechecked during 0.2.0 release preparation on
  2026-09-08: `cargo check -p flightlens` still fails at `libdbus-sys` because
  `pkg-config` is unavailable; the core-only `cargo check` passes.
  The session persistence added to
  `src-tauri/src/main.rs` has never been through a compiler: `cargo check -p
  flightlens` cannot run in the WSL checkout, where none of the Tauri Linux
  prerequisites in `DEVELOPMENT.md` are installed and `libdbus-sys` fails in its
  build script for want of `pkg-config`. This is a Linux-only obstacle. The
  desktop crate pulls 17 crates that need `pkg-config` and system headers on
  `x86_64-unknown-linux-gnu` and none on `x86_64-pc-windows-msvc` or
  `aarch64-apple-darwin`, because Tauri renders through WebKitGTK there and
  through WebView2 and WKWebView on the platforms FlightLens actually ships.
  Cheapest fix is to run the check on a Windows or macOS checkout, which needs
  no installs; installing the Ubuntu prerequisites also buys `pnpm tauri dev`
  for testing native picking, drop and restore by hand. Until one of those
  happens the release workflows are the first thing to compile the crate, which
  is late: a `v*` tag has already started publishing by then.
- Decide what to do about `pnpm format:check`, which fails on `src/App.tsx` and
  `tests/inspector.spec.ts`. Both deviations predate the current work; either
  reformat them in a commit of their own or drop them from the check.
- Relicense from GPL-3.0-or-later to MIT. The license is declared in `LICENSE`,
  `Cargo.toml`, `package.json`, `src-tauri/tauri.conf.json`, `README.md`,
  `DEVELOPMENT.md` and `THIRD_PARTY_NOTICES.md`. Settle the scope first: the
  bundled compatibility schemas and the rate equations are derived from
  Betaflight, and `src/assets/osd-default.mcm` is an unmodified resource from
  Betaflight Configurator, all GPL-3.0-or-later. MIT can cover FlightLens's own
  code, but the distributed application stays GPL unless that derived material
  is replaced or regenerated from a source that permits it.
- Add free security checks, release gating and status badges to CI. Nothing here
  costs anything on a public repository: `cargo audit` or `cargo deny` against
  the Rust advisory database, `pnpm audit` for JavaScript, CodeQL for Rust and
  TypeScript, `gitleaks` for secrets, and Dependabot for updates. This needs a
  workflow that runs on push and pull requests, which the repository does not
  have yet; today CI only runs on `v*` tags. Make the two release workflows
  depend on it so a tag with a failing check publishes no installer, and put the
  badges in the README.
- Sign the Windows installers. They are unsigned today, so SmartScreen warns on
  first run, and no free path clears that: it takes an OV or EV code-signing
  certificate or an Azure Trusted Signing subscription, both paid, and a
  self-signed certificate does not help. macOS is the same problem and stricter:
  an unsigned `.dmg`/`.app` is refused by Gatekeeper until the user opens it
  through the right-click override, and clearing that needs a paid Apple
  Developer Program membership for a Developer ID certificate plus notarization
  with `xcrun notarytool`. Decide whether to pay for either; until then document
  the override steps for both platforms in the README.

## Later

- Phase 2 of the roadmap: workspace indexing, file associations, two- and
  three-way comparison, saved `.flightlens` sessions.
- Signing and platform packaging for the first public release.

## Accepted limitations

These are deliberate. Reopen them only with a reason, not by habit.

- `pnpm screenshots:check` stays out of `./test.sh` and CI, because font
  rasterization differs between machines. Run it locally before a release.
- The 89-file real-backup corpus check needs a local checkout of
  [irom77/fpv_cli_dumps](https://github.com/irom77/fpv_cli_dumps) and is not
  reproducible in CI.
- Native picker, drag-and-drop, clipboard and packaged-app behavior need a
  graphical desktop; CI compilation does not cover them.
