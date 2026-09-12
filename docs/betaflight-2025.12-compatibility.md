# Betaflight 2025.12 compatibility investigation

Investigated 2026-09-11. This records the implementation checkpoints, not certification of the KAACK fork. Only public upstream source was fetched; no backup contents were uploaded.

## Available source pins

`git ls-remote --tags https://github.com/betaflight/betaflight.git 'refs/tags/*2025*'` returned RC1–RC4 and plain releases 2025.12.1 through 2025.12.5, but no 2025.12.0 tag. Use 2025.12.1 as the first plain release when extending the generator, rather than assuming a `.0` tag exists. Verified pins: 2025.12.1 = `85d201376a1fc33b223c27448808c2cc7b8f2743`; 2025.12.3 = `db7df6e48b9727d5984e18c906bf0e4769b2abf1`; 2025.12.5 = `7348054f268f0058574719c134e9f149565bb8ea`. [Official tags](https://github.com/betaflight/betaflight/tags)

## Serial syntax is a separate compatibility change

The 2025.12.3 CLI prints named ports in `serial` commands. It first resolves names case-insensitively, then accepts numeric identifiers; numeric values below 20 are translated from the legacy UART numbering. Thus simply normalizing names to numbers risks confusing the old and new numbering. [CLI printSerial and cliSerial](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/cli/cli.c#L1258-L1325)

Upstream identifiers include VCP = 20, SOFT1/2 = 30/31, LPUART1 = 40, UART0 = 50 where available, UART1 = 51, and PIOUART0 = 70. Names are build-dependent entries in `serialPortNames`. The function mask adds GIMBAL at bit 18 relative to 4.5.0. Preserve semantic port identity and firmware-appropriate export syntax; test named ports and legacy numeric input. [Serial identifiers and masks](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/io/serial.h), [Serial names and lookup](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/io/serial.c#L147-L223)

## Generator and analysis findings

The existing schema regex matches 679 setting declarations and 67 lookup-table entries at each of 2025.12.1, .3, and .5, versus 598 declarations and 63 tables at 4.5.0. The declaration format remains compatible with the generator's initial extraction, but this count does not prove every bound, enum or conditional setting is resolved. Profile counts in `common_pre.h` are four PID and four rate profiles at the three inspected 2025.12 tags. [Settings](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/cli/settings.c), [Lookup enum](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/cli/settings.h), [Profile counts](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/target/common_pre.h)

The four rate function bodies extracted by `tools/build_rate_vectors.py`—Betaflight, Kiss, Actual and Quick—are byte-identical between 4.5.0 and 2025.12.1. Extend and compile differential vectors before claiming support; this inspection does not verify the custom fork or every patch. [4.5.0 rate functions](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc.c), [2025.12.1 rate functions](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/fc/rc.c)

The rate reset function appears identical across inspected .1, .3 and .5 releases and includes `thrHover8`. The generator must still prove invariance across **all** plain patches before certifying omitted defaults. A suffixed custom/prerelease version must continue to receive no certified defaults: upstream source cannot establish what KAACK changed. [Rate reset](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/fc/controlrate_profile.c)

OSD coordinate packing retains the 4.5 layout: five low X bits, five Y bits, extra HD X bit at 10, visibility bits starting at 11 and type bits at 14–15. Other OSD enums changed, including timer sources and warnings, so coordinate compatibility does not certify all OSD interpretation. [2025.12.3 OSD definitions](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/osd/osd.h), [4.5.0 definitions](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/osd/osd.h)

## Serial checkpoint completed

Named ports, case-insensitive lookup, legacy numeric aliases, and the Gimbal mask
are implemented for 2025.12 inspection. Serial export token round trips are tested;
actual 2025.12 export remained gated until the schema checkpoint below. Full checks pass
against the current 14-file corpus, with all 15 original serial errors removed.

## Schema checkpoint completed

The bundled `betaflight-2025.12.1.json` now supplies 679 setting definitions and
four PID/rate profiles. Every schema input (including bound headers and external
debug names) was SHA-256 identical across 2025.12.1–2025.12.5; the pack records
all source URLs and hashes in `sources` and `schema_verified`. The generator
asserts this invariance on regeneration. All enum lists are populated. Signed
bounds, arithmetic expressions, and enum constants are resolved from upstream
headers; ambiguous conditional definitions are left unresolved.

Nineteen rate defaults were extracted from the reset function and verified at all
five plain releases. Compiled, unmodified upstream C produced 672 differential
rate vectors per release (3,360 new vectors). The source `fc/rc.c` SHA-256 is
`74396af807110de4905a9517426a9074eb521de43aab5d96bfc799e1882533f5` at all five
releases. Custom/prerelease builds receive the schema but no inferred defaults.

Twelve integer settings remain non-exportable, enumerated in the pack's
`unresolved_bounds`: seven full unsigned 32-bit fields exceed the current signed
integer model; OSD profile count, TPA low rate and VTX band/power bounds depend on
build options; the telemetry sensor mask still needs resolution. Hardware-specific
schema entries and array export remain outside this checkpoint. These gaps do
not block explicit rate inspection/export. Unknown or out-of-range values are
preserved as source text, never rewritten.

Validation passed: `./test.sh` against the 14-file Windows corpus, the expression
resolver tests (`python3 -B -m unittest discover -s tools -p 'test_schema_expressions.py'`),
and `git diff --check`. Both original 2025.12 custom backups now have four complete
rate profiles, four complete PID profiles, 79 OSD positions and successful rate
export, with no inferred defaults. Across the corpus, all 64 rate profiles and
52 PID profiles are complete. Thirty diagnostics remain; enabling schema checks
exposed two previously unchecked validation errors.

## Next checkpoint

The subsequent feature parser checkpoint accepts digits in names and only one
optional disable prefix. Synthetic regression coverage verifies `3D` enable/disable
state, last-command precedence, malformed commands, and OSD/serial export. This
removes all 14 feature syntax errors, reducing the corpus total from 30 to 16.

Resolve the remaining schema validation diagnostics recorded
in TODO.md. Extend the explicitly deferred bound/value-model coverage separately;
do not substitute a guessed target build for conditional definitions.

## Conditional LED lookup checkpoint

The generator kept only the last definition of each C lookup table. The LED
profile table's shorter conditional definition overwrote the version containing
`STATUS`. Extraction now keeps the widest definition when shared ordinals agree
and fails on conflicting ordinals rather than silently changing default mappings.
Regeneration changes only the LED choices in all five packs. Build feature
availability remains uncertified; accepting a declared choice does not establish
that every destination build provides it.

Synthetic parser coverage reproduces the rejection on every supported line.
The private corpus probe reports only parameter names, counts, and a bound-check
boolean, never source values. Fourteen LED errors are removed; two diagnostics
remain. The GPS value satisfies the exact 4.4.3 bound (10–30), whereas the base
4.4.0 pack uses 20–1000. This requires patch-aware schema handling in the next
checkpoint. The OSD units choices are unchanged between the base 4.5.0 and exact
4.5.2 source, so that diagnostic remains valid against upstream.

Sources: [4.4.3 settings](https://github.com/betaflight/betaflight/blob/4.4.3/src/main/cli/settings.c),
[4.5.2 settings](https://github.com/betaflight/betaflight/blob/4.5.2/src/main/cli/settings.c).
All regenerated packs retain their recorded upstream source hashes.

### Betaflight 4.4 GPS rescue patch checkpoint

The upstream `gps_rescue_min_start_dist` declaration uses 20–1000 in
[4.4.0](https://github.com/betaflight/betaflight/blob/4.4.0/src/main/cli/settings.c)
and [4.4.1](https://github.com/betaflight/betaflight/blob/4.4.1/src/main/cli/settings.c),
then 10–30 in
[4.4.2](https://github.com/betaflight/betaflight/blob/4.4.2/src/main/cli/settings.c)
and [4.4.3](https://github.com/betaflight/betaflight/blob/4.4.3/src/main/cli/settings.c).
The generator records those per-patch schemas with source URLs and SHA-256 hashes.
Pack selection applies verified overrides to that patch and its build suffixes;
unverified patches retain the base schema. Custom defaults remain uncertified.
Boundary regressions cover both accepted endpoints and adjacent rejected values.
