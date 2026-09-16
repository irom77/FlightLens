# PID default recovery evidence

D1 completed on 2026-09-16 for a bounded set of **ten build-invariant gains** on
plain official Betaflight **4.5.0–4.5.5**. Runtime recovery, PID display,
comparison and export-exclusion checks are implemented. Other firmware lines,
vendor builds, roll/pitch D, and D-min/D-max remain outside this certification.

## Recovered values and source contract

| Axis | P | I | D | F |
| --- | --- | --- | --- | --- |
| Roll | 45 | 80 | Not recovered | 120 |
| Pitch | 47 | 84 | Not recovered | 125 |
| Yaw | 45 | 80 | 0 | 120 |

A plain release identity is interpreted as the unmodified upstream release
source and build machinery, the same identity contract used for rate defaults.
It is not cryptographic authentication of a firmware binary: a custom build
claiming an official identity cannot be detected from the version string alone.
Arbitrary injected C, changed headers, target callbacks and modified default
macros are outside the contract. Known vendor/prerelease suffixes are excluded.

The ten selected constants are unconditional in upstream headers. Board and
feature selection do not change their reset assignments. Complete release
archive scans find no implementation of `targetConfiguration` in any of the six
releases. `mk/config.mk` selects an external board `config.h`; it does not add an
external C implementation. Defining `USE_TARGET_CONFIG` without adding code
cannot supply the missing callback. A board configuration that injects its own
implementation or changes upstream definitions is a custom source build.
[Reset implementation](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/src/main/flight/pid.c),
[default constants](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/src/main/flight/pid.h),
[configuration selection](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/mk/config.mk).

This is why the selected table does not require a board name or `# config rev:`.
The earlier single-board preprocessing experiment was useful evidence of one
build, but cannot establish provenance for historical backups. Recovery now
uses only fields invariant across the feature choices instead of extrapolating
that snapshot to all boards.

## Reset, omission and tuning paths

The verified CLI omission path is `cliDiff` → `printConfig` →
`backupAndResetConfigs` → `resetConfig`. `dumpAllValues`/`dumpPgValue` compare
saved parameter groups against the reset groups. This omission baseline does
**not** run simplified tuning. The `defaults` command additionally calls
`applySimplifiedTuningAllProfiles` when enabled.
[CLI paths](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/src/main/cli/cli.c),
[reset sequence](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/src/main/config/config.c).

The selected ten values agree in both paths. Default P/I/F multipliers are
100/100, so their factors and integer products are exactly representable; yaw D
is zero. They avoid the D-min ratios responsible for uncertain roll/pitch D.
`resetPidProfile` assigns roll/pitch D as 40/46 with `USE_D_MIN`, but replaces
these with 30/32 without it. These two fields are not recovered.
The full tuning source's no-D-min branch refers to a conditionally absent
`dMinDefaults`; the harness does not patch that invalid build combination or
claim to execute it. It checks reset without D-min, and tuning with D-min.
[Tuning implementation](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/src/main/config/simplified_tuning.c).

Non-default tuning is not simulated. The independent reduced C harness tests
4,824 slider/mode profiles at each optimization level; O0/O2 match, while
Ofast differs in 270 axis rows on GCC 13.3.0. This is a concrete reason not to
recover gains following `simplified_tuning apply`. The command affects all
profiles. `disable` changes tuning modes and filters but does not restore gains.

## Reproducing the evidence

These are developer-only tools. Network verification downloads public source
only, never backups, and never runs in the offline application.

- `python3 tools/verify_pid_defaults.py --check`: verifies immutable source
  hashes, twelve CLI mappings, six default macros and the reset/CLI/tuning
  functions across six releases. Source equality alone is not certification;
  its `certifiedForRecovery: false` flag intentionally records that limitation.
- `python3 tools/verify_pid_builds.py --check`: downloads the six immutable
  release archives, verifies the preceding source hashes against them, scans
  build sources for default-macro locations and target callback implementations,
  and checks the generated runtime table in
  `crates/flightlens-core/compatibility/pid-defaults.json`. The table's ten values
  are extracted from the pinned initializer evidence, not maintained separately.
  Each archive's relevant source-tree manifest digest is recorded.
- `python3 tools/check_pid_reset.py SOURCE`: checks every source/header hash,
  compiles complete upstream `resetPidProfile` and `pgResetFn_pidProfiles` with
  real `pidProfile_t` and SITL headers, and runs upstream
  `pgResetAll` → `pgReset` → `pgResetInstance` with a PID-only host registry.
  All four profiles are reset from zero and nonzero memory. It tests reset with
  D-min, reset without D-min, and reset plus default simplified tuning, each at
  O0/O2/Ofast with undefined-behavior and float-cast-overflow sanitizers.
- `python3 tools/check_pid_tuning.py SOURCE`: runs the reduced non-default
  arithmetic sweep described above.

`SOURCE` is an extracted checkout of firmware commit
`c155f5830d0ffdee1c34071dd21f174ffc374c81`. The C tools need a GCC-compatible
compiler (`CC`, default `cc`); the reset tool also needs a GNU-compatible linker.
They exercise host execution, not hardware linker layout or an embedded binary.
The selected constants and exact default P/I/F arithmetic, combined with the
source/build verification, establish the limited table; host execution alone
would not establish arbitrary embedded tuning equivalence.

The earlier `tools/check_pid_hardware.py FIRMWARE_SOURCE CONFIG_SOURCE --check`
experiment remains reproducible. It preprocesses SPEEDYBEEF405V4 against config
commit `7b1f01a25d8cb6379ebeeeca9f0e91c24925e907` with real STM32F405/CMSIS headers:
44 pinned inputs, D-min and tuning enabled, four profiles, no target callback.
It neither selects the runtime table nor authenticates historical configurations.

## Runtime boundaries

Recovery requires a valid final `defaults` command and only covers profiles
actually seen in the document within the pack's bounds. No untouched profile is
invented. Explicit settings always win, including invalid values and zero.
Assignments with unknown scope cannot be replaced by a guessed default.

After that reset, an unmodeled `simplified_tuning` command, an ambiguous
reset/profile/assignment, or an out-of-range profile selector blocks PID
recovery. A later valid reset establishes a fresh baseline. Exact `disable`
(case-insensitive argument) preserves recovery, but does not undo an earlier
`apply`. Merely assigning tuning sliders does not apply them. Source statements
remain declared statements; FlightLens does not simulate effective command or
boot-time corrections.

Recovered gains use the existing `Derived` model. PID cards count them, cells
show an asterisk and firmware origin, and the separate defaults table says
“not declared, not exported.” Recovered cells do not link to a fictitious source
line. Two-/three-way parameter comparison uses those same values and retains
unknown/invalid distinctions. PID snippets still include only declared settings.
Corpus coverage counts recovered values, but a ten-gain profile is still
incomplete without explicitly declared roll/pitch D.

Tests cover all six releases, unsupported identities, reset/command ordering,
profile boundaries, zero/invalid declarations, export exclusion, actual Rust
DTO comparison, renderer provenance, profile switching and declared source jumps.
Further firmware/build certification and non-default tuning replay are separate
coverage extensions recorded in `TODO.md`.

## Validation on 2026-09-16

`./test.sh` passed (Rust formatting/Clippy, core tests, generated bindings,
ESLint, TypeScript, 85 frontend tests, 29 browser tests and local corpus checks).
The final core run passed all 99 tests, including the added corpus metric and
BOM command regressions.
All 14 native tests, both screenshot checks, production build and formatting
checks passed as well; existing screenshot baselines required no changes.
The reset harness passed all nine feature/optimization combinations, and the
six-archive runtime-table check and six-release source check passed.

On the same 97 local backups, PID profiles with known gains increased from 104
to 171, and complete twelve-gain profiles from 23 to 47, with zero corpus
failures. These are aggregate coverage results; no backup contents are included
in this document or the public-source verification tools. Three pre-existing
parse errors in corpus inputs remain reported rather than repaired or hidden.
