# Throttle curve preview source review

Implemented, 2026-09-12. The Rust calculation is in
`crates/flightlens-core/src/throttle.rs`, with documented-vector, release-gate,
input-validation and profile-isolation tests. Compiled upstream C differential
validation and inspector integration are complete. Browser coverage checks
profile switching, source links, unsupported firmware, missing input and MID=100.

## Recommended first implementation

Plot normalized throttle input against configured throttle command for exact
reviewed official releases **4.2.0, 4.3.0, 4.4.0, and 4.5.0–4.5.5**. Require a
compatible Betaflight schema pack and explicit, valid values from the selected
rate profile. Do not infer support for other patch releases, vendor suffixes,
unknown versions, iNav, or ArduPilot. Missing settings remain unknown rather than
silently using firmware defaults.

Keep **2025.12.1–2025.12.5 unavailable initially**: their curve uses `thr_hover`
and a different construction and interpolation algorithm. Adding hover=50 to
the legacy formula would still be incorrect. This is a separate implementation
follow-up, not a reason to delay the finite legacy preview.

For the legacy preview, reject `thr_mid=100` with an explicit unsupported-edge
reason: upstream initializes an extra lookup knot beyond the input domain with a
zero divisor. The CLI accepts this value, so distinguish the preview limitation
from invalid configuration. Supporting its idealized mathematical extension would
need separate labeling and should not be presented as verified firmware behavior.

## Settings and profile scope

| Setting | Stored field | CLI range or enum | Scope |
| --- | --- | --- | --- |
| `thr_mid` | `thrMid8`, uint8 | 0–100 | Rate profile |
| `thr_expo` | `thrExpo8`, uint8 | 0–100 | Rate profile |
| `throttle_limit_type` | `throttle_limit_type`, uint8 | OFF=0, SCALE=1, CLIP=2 | Rate profile |
| `throttle_limit_percent` | `throttle_limit_percent`, uint8 | 25–100 | Rate profile |
| `thr_hover` (reviewed 2025.12 only) | `thrHover8`, uint8 | 0–100 | Rate profile |

These are `PROFILE_RATE_VALUE` declarations in `PG_CONTROL_RATE_PROFILES`, not
PID-profile or global settings. The enum and storage are in
[4.5.0 controlrate_profile.h](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/controlrate_profile.h).
Declarations were checked in every listed release; representative sources are
[4.2.0 settings](https://github.com/betaflight/betaflight/blob/4.2.0/src/main/cli/settings.c),
[4.5.0 settings](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/settings.c),
and [2025.12.1 settings](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/cli/settings.c).

## Legacy lookup and integer behavior

Let `m=thr_mid`, `e=thr_expo`. For each knot index `i`, define `d=10*i-m`.
Use `s=100-m` when `d>0`, `s=m` when `d<0`, and `s=1` at `d=0`.
The normalized command knot in thousandths is:

```text
q[i] = 10*m + trunc(d * (100-e + trunc(e*d*d/(s*s))) / 10)
```

Each truncation is C integer division toward zero. In particular, a negative
outer quotient must not use floor division. The firmware stores `1000+q[i]`
in an int16 lookup. It builds 12 knots (`i=0..11`), with the last at 110% input.
At `m=100`, that last knot divides by zero; this is why the first preview should
reject that edge. The regular domain uses knots 0..10.

For integer normalized input `t=0..1000`, take `k=trunc(t/100)` and interpolate:

```text
q(t) = q[k] + trunc((t-100*k)*(q[k+1]-q[k])/100)
```

At `t=1000`, return `q[10]` directly in the preview; the extra knot has zero
weight. Sample integer inputs (for example all 1001 values), rather than plotting
a continuous cubic formula: the firmware interpolates quantized knots. With
`e=0`, the result is linear for every supported midpoint. The supported curve
has endpoints 0 and 1000 before limits. UI percentages are `t/10` and `q(t)/10`.
Sources: [4.3.0 rc.c](https://github.com/betaflight/betaflight/blob/4.3.0/src/main/fc/rc.c),
[4.4.0 rc.c](https://github.com/betaflight/betaflight/blob/4.4.0/src/main/fc/rc.c),
[4.5.0 rc.c](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc.c).

The construction is identical across reviewed legacy releases apart from
`PWM_RANGE_MAX-PWM_RANGE_MIN` becoming `PWM_RANGE`; lookup interpolation is
identical. This assertion is about these functions and tags, not entire firmware.

## Limits and the meaning of the axes

Apply the configured limit **after** the curve. For command fraction `u=q/1000`
and fraction `p=throttle_limit_percent/100`, OFF returns `u`, SCALE returns `u*p`,
and CLIP returns `min(u,p)`. A limit of 100% leaves the curve unchanged. The
mixer uses floating-point arithmetic for this step; do not introduce another
integer truncation. Use a numerical tolerance when comparing JavaScript double
results with firmware float results.

In 4.5 the limit is bypassed while `RPM_LIMIT_ACTIVE`; 4.2.0, 4.3.0, and 4.4.0
have no such guard in `applyThrottleLimit`. Thus a chart with limits represents
the configured ordinary path, not every runtime mode. Sources:
[4.4.0 mixer.c](https://github.com/betaflight/betaflight/blob/4.4.0/src/main/flight/mixer.c),
[4.5.0 mixer.c](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/flight/mixer.c),
[2025.12.1 mixer.c](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/flight/mixer.c).

The horizontal axis should describe **normalized throttle input**, not raw
receiver microseconds. The legacy RC path constrains ordinary input using
`mincheck` and 2000 and rescales to 0..1000; low-voltage cutoff can scale that
input before lookup. 3D mode changes normalization and subsequent handling.
RC smoothing can affect the signal entering the mixer. Throttle boost occurs
after the limit and can raise the result; dynamic idle, thrust linearization,
airmode, PID mixing, rescue, and motor output configuration add further behavior.
These are outside a static preview. Label the vertical axis **throttle command
(%)**, not motor output, thrust, power, RPM, or current. Provide one concise
explanation that the chart shows configured curve/limit behavior and excludes
runtime effects. No prediction of actual aircraft output is justified.

## Why 2025.12 needs a separate model

At all five reviewed 2025.12 tags, the lookup uses two quadratic Bézier segments
through `(0,0)`, `(thr_mid/100,thr_hover/100)`, and `(1,1)`. Expo changes the
control-point heights. Twelve knots are spaced across the entire 0..1 interval
(increments of 1/11), rounded with `lrintf` after mapping to the PWM range.
Lookup interpolation scales input by 11, uses quotient/remainder over 1000, and
clamps the upper index. This changes both curve shape and quantization.
[2025.12.1 rc.c](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/fc/rc.c)
and [2025.12.5 rc.c](https://github.com/betaflight/betaflight/blob/2025.12.5/src/main/fc/rc.c).

A future implementation must review the complete `quadraticBezier` and scaling
helpers, degenerate midpoints, float precision and `lrintf` rounding, and require
explicit hover. That helper-level certification is outside this checkpoint.
The five releases have identical normalized lookup construction and interpolation
bodies; this alone does not certify a new preview implementation.

## Regression vectors and validation

The following independently evaluated legacy vectors are command thousandths
before limits. Use them alongside a C reference extracted from the reviewed
functions; retain C integer operations when compiling that reference.

| mid | expo | input 0 | 100 | 250 | 300 | 500 | 750 | 900 | 999 | 1000 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 50 | 0 | 0 | 100 | 250 | 300 | 500 | 750 | 900 | 999 | 1000 |
| 50 | 50 | 0 | 172 | 340 | 384 | 500 | 660 | 828 | 998 | 1000 |
| 50 | 100 | 0 | 244 | 430 | 468 | 500 | 570 | 756 | 997 | 1000 |
| 30 | 70 | 0 | 178 | 281 | 300 | 370 | 566 | 786 | 997 | 1000 |
| 0 | 100 | 0 | 1 | 17 | 27 | 125 | 427 | 729 | 997 | 1000 |

For mid=50/expo=50/input=750, OFF gives 66%, SCALE 50 gives 33%,
and CLIP 50 gives 50%. These distinguish applying limits after lookup from
incorrectly limiting input before lookup. Include endpoint, knot-neighbor,
negative-division, invalid/missing/profile-selection, unsupported-version,
mid=100-unavailable, and OFF-with-valid-unused-percent tests. Do not clamp an
invalid configuration silently into range.

Research downloaded only public source files, five per exact tag:
`src/main/fc/rc.c`, `src/main/fc/controlrate_profile.h`,
`src/main/fc/controlrate_profile.c`, `src/main/cli/settings.c`, and
`src/main/flight/mixer.c` (70 files). No backup data was used or transmitted.
Reproduce the numerical fixtures with `python3 tools/build_throttle_vectors.py`
(requires Python, a C compiler, and network access to public upstream source).
The developer tool extracts the unmodified lookup initialization loop,
`rcLookupThrottle`, and `applyThrottleLimit` at the nine immutable legacy commits
below, compiles them with undefined-behavior sanitization, and records source
SHA-256 hashes in `fixtures/throttle-vectors.json`. A minimal shim supplies the
reviewed constants/profile fields and selects the ordinary non-RPM-limited path.
It does not compile the entire firmware or independently verify CLI declarations.

The checked-in fixture contains 28,512 vectors covering eight midpoint/expo
pairs, all three limit modes, four percentages, endpoints, lookup knots and their
neighbors, and intermediate inputs. The Rust differential test consumes these
fixtures offline, checks exact pre-limit integer results, and allows 0.00002
percentage points for the C float limit result. Run it with
`cargo test -p flightlens-core --test throttle`. The midpoint=100 exclusion is
covered separately by an unavailable-state test, not executed in the C harness.

## FlightLens integration boundary

All four legacy settings already exist in the five bundled compatibility packs
as rate-profile settings. Use `ConfigDocument::parameter` in
`crates/flightlens-core/src/model.rs` to require declared, valid, supported inputs;
the initial preview deliberately does not use `number_or_default` or
`text_or_default`. Existing rate-curve default recovery remains independent.
Require an exact reviewed firmware identity and its matching pack before
calculating a curve. Display missing, invalid, unsupported-release, and midpoint
edge reasons separately from an available result.

The selected rate profile and inspection request already flow through
`src/App.tsx` and `crates/flightlens-core/src/analysis.rs`. Keep the calculation
in the Rust analysis layer and expose its result through generated bindings
(`crates/flightlens-core/src/bin/bindings.rs`). Display imported MID, EXPO and
limit values as read-only fields with source provenance; profile selection is
the control that changes which imported curve is shown. Do not add editing or
temporary tuning behavior to this slice.

The existing `src/Plots.tsx` axes assume signed stick input and angular velocity
or filter frequency, so the throttle chart needs explicit 0–100% axes and its own
accessible description. Keep source navigation and existing rate plots working.
Use the upstream C-fixture approach in `tools/build_rate_vectors.py` for an
independent numerical reference. Validate parser-to-preview profile switching,
unavailable states and labels in browser coverage, then run `./test.sh` and
`pnpm screenshots:check` for the completed feature checkpoint. The inspection DTO includes the throttle curve through regenerated TypeScript
bindings. The inspector shows imported values with source links and a percentage
plot; it does not offer tuning or editing controls.

## Exact upstream revisions

Tags resolved with `git ls-remote --tags https://github.com/betaflight/betaflight.git`
on 2026-09-12. Use the commit in source URLs for immutable reproduction.

| Tag | Commit |
| --- | --- |
| 4.2.0 | `8f2d21460a9913d58bd1c33f8348c3791451fb45` |
| 4.3.0 | `229ac667552827c6288550964a0ef877c04bf8ab` |
| 4.4.0 | `4605309d8253db0113d4c54d31fe8bd998f46401` |
| 4.5.0 | `c155f5830d0ffdee1c34071dd21f174ffc374c81` |
| 4.5.1 | `77d01ba3b76a22909d5f09cb0628820141f95eaa` |
| 4.5.2 | `024f8e13d4e642eb6a380308685b9ea3aa3ef1a2` |
| 4.5.3 | `0e533ba76cb9129458578f7fa2134df1449596ba` |
| 4.5.4 | `25356b59a96dc975d777844f896b7c7855ea90e6` |
| 4.5.5 | `4adbd3ef7cb546947600e5f747bd5453c9573063` |
| 2025.12.1 | `85d201376a1fc33b223c27448808c2cc7b8f2743` |
| 2025.12.2 | `79065c96ba0bb5cdc675e67d7093e05dab8b330e` |
| 2025.12.3 | `db7df6e48b9727d5984e18c906bf0e4769b2abf1` |
| 2025.12.4 | `c2af58a0cbe060bd44ee2eb53baf0f42baf7d15e` |
| 2025.12.5 | `7348054f268f0058574719c134e9f149565bb8ea` |
