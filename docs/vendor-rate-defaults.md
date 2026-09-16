# Verified vendor rate defaults

D2 uses exact source certification rather than an opt-in guess. After a valid
`defaults` baseline, FlightLens recovers omitted rate-profile settings for:

| Reported version | Pinned vendor commit | Matching upstream pack |
| --- | --- | --- |
| `4.5.3.KAACK_V19` | [`8cd44381217948c0b2b5087f12e17dde15d6a25c`](https://github.com/limonspb/betaflight/tree/8cd44381217948c0b2b5087f12e17dde15d6a25c) | 4.5.0 |
| `2025.12.3-alpha.KAACK_V19` | [`3b419ca431ba5ea791d7924c8c5b266b911da5b1`](https://github.com/limonspb/betaflight/tree/3b419ca431ba5ea791d7924c8c5b266b911da5b1) | 2025.12.1 |

These are the public vendor revisions already identified in the
[throttle source investigation](throttle-curve-preview.md#exact-kaack-source-discovery--2026-09-14).
Other suffixes, patches, V18/V20, or an additional custom suffix do not qualify.
The firmware version is preserved; recovered values name the exact vendor
version in their provenance, not an upstream release presented as that build.

## Evidence

[`tools/verify_vendor_rate_defaults.py`](../tools/verify_vendor_rate_defaults.py)
downloads immutable archives for both vendor commits and their pinned upstream
baselines. It reads no controller backups. The verifier checks:

- Version-header definitions matching the exact advertised vendor identities.
- Identical complete rate reset functions, with the upstream reset hash anchored
  to the bundled compatibility pack.
- Identical control-rate struct and enum definitions, axis indexes, reset macro,
  parameter-group reset implementation and declarations.
- Identical CLI `defaults` and configuration reset functions.
- Identical rate-setting field mappings, CLI name definitions and relevant enum
  lookup arrays, including ACTUAL, throttle limit OFF and quick-rates expo OFF.
- Identical rate-limit constant and profile-count alternatives. The source scan
  finds only one definition of the rate-limit constant and no implementation of
  a target configuration callback in either vendor archive.

The generated [offline evidence table](../crates/flightlens-core/compatibility/vendor-rate-defaults.json)
records source/archive hashes, pinned commits, the source manifest hash, reset
hash and recovered values. Runtime also checks that its pack version, reset hash
and complete value map match the certified vendor entry. Regenerating a pack
with different values therefore cannot silently reuse the vendor certification.

There are 18 recovered settings per otherwise empty observed profile on 4.5 and
19 on the year-based build (which adds `thr_hover`). Both reset to ACTUAL rates,
RC rate 7, super rate 67, expo 0 and rate limit 1998 on every axis. Throttle MID
is 50, EXPO 0, throttle limit OFF/100, quick-rates expo OFF; year-based throttle
hover is 50.

Reproduce the public-source check:

```sh
python3 tools/verify_vendor_rate_defaults.py --check
```

Omit `--check` only when intentionally regenerating the table, then review the
result. The application never fetches this evidence or accesses the network.

## Runtime limits

This certification assumes the reported exact version identifies the pinned
vendor source and its normal build machinery. It does not authenticate a flashed
binary or a modified build reusing that identity, nor certify arbitrary injected
source, overridden headers or external target callbacks. This is the same source
identity trust boundary as official release recovery.

Recovery requires a parsed reset baseline. Only profiles observed after the final
reset are considered, including profile 0 selected by reset. Explicit values
always win, even zero or invalid values. A malformed or unmodeled reset,
rate-profile selector or assignment after the baseline prevents rate recovery;
a later valid reset starts a new baseline. Recovery does not replay arbitrary
commands or simulate boot-time validation.

The rates inspector marks recovered inputs with `*`, lists their vendor
provenance, and can plot the resulting curves. Same-version comparisons include
these values and preserve declared-versus-recovered provenance. This does not
certify new cross-version equivalence. Export continues to use explicit settings
only; recovery cannot make an incomplete rate snippet exportable.

This certifies only the rate-profile reset table. Vendor PID defaults remain
unknown. The throttle preview still requires explicit input declarations, even
when the rate-profile table contains recovered throttle values; enabling omitted
throttle inputs is tracked separately in TODO.md. Target-specific baselines and
other vendor versions are not inferred.

## Validation

Core tests cover both exact versions, complete curves, independent profiles,
explicit zero and invalid inputs, missing/final reset behavior, ambiguous
commands, exact-identity rejection and export exclusion. Renderer tests use real
Rust fixture output to check both vendor versions' curves, `*` labels and
provenance, alongside an unverified vendor and explicit zero. Comparison tests
cover declared versus recovered equality and three-way changes on both builds.

On the same 97-file local corpus, complete selected rate profiles increased from
77 to 89, and complete rate profiles overall from 349 to 397. There were zero
corpus failures; the three pre-existing parse errors remain. All 52 observed rate
profiles across the 13 backups with these exact vendor identities are complete. No backup was
modified or sent to an external service.
