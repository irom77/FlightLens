# TODO

Outstanding work on FlightLens. Items move out of this file when they ship: a
user-visible one becomes a `CHANGES.md` entry, an internal one just disappears
with the commit. Long-range direction lives in [ROADMAP.md](ROADMAP.md); this
file holds only what is actionable now.

## Next

- Implement Phase 2 item 4: three-backup comparison with an explicit baseline,
  independent profile selections, angular velocity overlays, and separate
  one-sided changes and conflicts. Reuse existing compatibility and unknown-value
  rules; keep the comparison read-only. Item 3 is complete within its fixed
  [roadmap boundary](ROADMAP.md); broader mapping research is deferred.

- Develop a guide explaining how FlightLens processes Betaflight (BF) CLI dumps,
  building on the existing research and comparison documentation in `docs/`.
  Cover firmware/version detection, parsing and profile scopes, schema selection
  and validation, raw/unknown data preservation, comparison and equivalence rules,
  diagnostics, and export limits. Link to the existing compatibility, parameter
  equivalence, and CLI collection documents for detailed evidence, and distinguish
  implemented behavior from deferred support. Organize the guide with a shared
  processing overview and a Betaflight section so iNav and ArduPilot (AP) sections
  can be added later without implying current support.

- Complete 2025.12 export coverage for the 12 settings listed in the bundled
  pack's `unresolved_bounds`: build-dependent OSD/TPA/VTX limits, telemetry sensor
  masks, and unsigned 32-bit settings. The current integer model is signed 32-bit;
  source values outside it remain raw and cannot be exported. Hardware-specific
  schema entries and arrays also remain outside certified export coverage.
- Keep the remaining corpus `osd_units` diagnostic: the value is outside the
  exact upstream 4.5.2 choices. Revisit only with evidence of additional syntax.
- Extend patch-specific schema verification beyond the known 4.4 GPS rescue
  bound change; unverified patches continue to use the base firmware-line schema.
- Manually check the new comparison view in packaged Windows/macOS builds.
- Manually check the feedback hand-off in packaged Windows/macOS builds, including
  the upfront GitHub account notice, attachment toggles during preview preparation
  and near-limit file guidance. The
  prefilled issue must open in the default browser and the redacted
  configuration must reach the system clipboard. Only the Linux/WSL path has
  been exercised.
- Complete native restore checks on Windows and macOS, including unreadable or
  disconnected sources, and check native picking, dropping, clipboard and snippet
  saving. Linux/WSL restore smoke checks passed with synthetic backups on
  2026-09-08: duplicate paths stay closed after restart, changed files are reread,
  and missing files are reported once and removed from the saved list. Unreadable
  files report an error, stay saved, and reopen after permissions are restored.
  Linux native picking, clipboard export, snippet saving and save cancellation
  also passed, as did X11 file drag-and-drop from a GTK source. Wayland and
  platform file-manager integrations remain unverified. See
  [the validation record](DEVELOPMENT.md#native-restore-validation).
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

- After three-backup comparison, complete Phase 2 workspace indexing, portable
  `.flightlens` sessions, native file associations, and desktop validation in
  roadmap order. Saved comparison selections belong to the session work.
- Expand comparison coverage after Phase 2: additional parameter mappings,
  certified cross-version features/ports/modes and CLI collections, and additional
  rxrange/rxfail firmware coverage. These do not block Phase 2 item 3 completion.
- Extend [VTX activation comparison](docs/vtx-comparison.md) with verified build
  evidence for build-specific selector bounds and 2025.12 activation capacity.
- Investigate adjustment-range runtime equivalence separately from the completed
  [declaration comparison](docs/adjrange-comparison.md). Collection diagnostics
  and export remain separate follow-up work for all supported collections.
- Keep hardware-classified `vbat_scale` deferred until its schema support is verified;
  see [the voltage calibration review](docs/voltage-calibration-equivalence.md).
- Keep hardware-classified `ibata_scale` deferred pending schema support; see
  [the current calibration review](docs/current-calibration-equivalence.md).


## Accepted limitations

These are deliberate. Reopen them only with a reason, not by habit.

- `pnpm screenshots:check` stays out of `./test.sh` and CI, because font
  rasterization differs between machines. Run it locally before a release.
- The 89-file real-backup corpus check needs a local checkout of
  [irom77/fpv_cli_dumps](https://github.com/irom77/fpv_cli_dumps) and is not
  reproducible in CI.
- Native picker, drag-and-drop, clipboard and packaged-app behavior need a
  graphical desktop; CI compilation does not cover them.
