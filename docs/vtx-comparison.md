# VTX comparison: verified scope

Source review is complete for the initial comparison design. Explicit table and
official 4.5 activation comparison are implemented locally. The collection view
also retains preserved source text.

Run `python3 tools/verify_vtx_comparison.py` to regenerate
[source evidence](vtx-source-evidence.json). It checks 99 upstream files across
4.5.0–4.5.5 and 2025.12.1–2025.12.5 against pinned release-family baselines.
Nine CLI functions have normalized section hashes; eight supporting files have
normalized whole-file hashes. Whole-file byte hashes and URLs are also recorded.
This developer tool uses the network; the application remains offline.

## Findings

The [CLI handlers and dump functions](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/cli.c)
show that command order matters:

- `vtxtable bands`, `channels`, and `powerlevels` set dimensions. Shrinking
  dimensions clears affected storage. Increasing them does not restore it.
- Band declarations require the current band/channel counts. Power arrays require
  the current power-level count and exact operand counts.
- Names and labels are uppercased; names truncate to eight characters, labels to
  three. Band letters use the first character. Omitted FACTORY/CUSTOM means CUSTOM.
- Frequency and power storage is unsigned 16-bit; CLI integer parsing is permissive.
- Bare commands query. There is no dedicated reset subcommand.
- `vtx` takes slot, AUX index, band, channel, power, start and end. Band/channel/power
  validation uses current table dimensions when table support is compiled in.
  Invalid arguments can clear a slot. Channel-only invocation is not a query.
- Range inputs clamp to 900–2100 and quantize down to 25 µs steps. Extra operands
  are not consistently rejected by the upstream commands.

The [table helpers](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/drivers/vtx_table.c)
clear removed frequencies to zero, restore generated band names/letters and
CUSTOM flags for removed bands, and reset removed power values and labels.
Strings are padded with spaces. Table limits are eight bands, eight channels,
and eight power levels with table support; the
[non-table build constants differ](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/drivers/vtx_table.h).
A table power value must not be presented as a verified milliwatt measurement.

The [activation runtime](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/io/vtx_control.c)
depends on receiver state, device presence, previous selection, and arming lock.
Zero band/channel/power fields leave those settings unchanged when activated.
Equal slot declarations therefore do not establish equal RF output or behavior.

## Release differences and limits

Reviewed CLI sections are identical within each release family. The only
cross-family CLI difference is local variable scoping in `cliVtx`.
The 2025.12 table helpers add NONSTRING annotations and make an internal helper
static; reviewed clearing behavior is unchanged. Other supporting-header changes
include unrelated mode/receiver declarations.

Crucially, [2025.12 slot capacity](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/io/vtx_control.h)
can be overridden at build time; ten is the default, not a universal capacity.
The version header and schema pack alone do not establish build options or
hardware capabilities. No cross-version or vendor certification follows from
matching source sections. Driver-specific power interpretation and hardware
behavior have not been certified by this review.

## Implemented: explicit table comparison

- Gate on identical verified official version, lowercase `betaflight`, and the
  matching bundled schema pack. Keep vendor and cross-version results not comparable.
- Track explicit dimensions, bands, power values and labels in command order.
  Require known dimensions before accepting dependent declarations. Unknown initial
  state and defaults remain unknown; do not populate firmware defaults.
- Start with canonical dump syntax: exact operand counts, unsigned decimal values
  in storage range, ASCII names/labels within their stored lengths, one-character
  letters, and explicit FACTORY/CUSTOM. Normalize ASCII case and numeric spelling.
  Reject permissive parsing, overflow, and truncation from the certified subset.
- Conservatively invalidate dependent knowledge when dimensions change, rather
  than presenting inferred clearing defaults as explicit declarations. Repeated
  identical dimensions may preserve knowledge. Later complete declarations restore it.
- Zero-channel band declarations with an explicit flag are outside the certified
  subset because the CLI does not consume the flag when channel count is zero.
- Defaults or unverified table mutations clear table knowledge. Bare queries preserve
  it. Missing or invalidated entries stay unknown. Keep raw occurrences accessible.
- Display comparison rows for dimensions, each band and the power arrays, with
  filtering, source lines and hide-equal controls. Do not label power values as mW.
- Cover ordering, shrinking then growing dimensions, repeated declarations, empty
  tables, unknown dimensions, resets, malformed syntax, case normalization and
  firmware gates. Include a browser test using actual Rust parser output.

## Implemented: activation declarations

Slots are compared separately, preserving AUX index, band/channel/power selectors
and normalized ranges. The implementation follows the prerequisite decision below,
including unknown dependencies, invalid partial assignments, zero selectors,
overlapping slots and non-table builds. Matching selectors do not certify RF equivalence.
Core diagnostics and export remain outside these comparison slices.

## Activation prerequisite decision

The initial activation implementation will cover matching official 4.5.0–4.5.5
with their expected schema pack. The reviewed 4.5
[capacity declaration](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/io/vtx_control.h)
fixes ten slots. In 2025.12 the corresponding macro is overrideable; its upper
bound of 99 does not prove that any particular slot is allocated. Keep 2025.12,
vendor versions, other versions and cross-version pairs not comparable initially.
Do not infer capacity from the highest slot seen, contiguous rows, a board name,
or a `dump all` marker. Partial, edited and concatenated inputs are supported.

FlightLens's `Firmware` model currently records family, version, board name,
header and schema pack. It carries no verified VTX build capabilities. A schema
pack is not such evidence, and a `vtxtable` declaration alone does not prove that
the build implements it. No new build-evidence field is needed for the narrow
4.5 slice below. Extending capacity certification requires separately verified
build metadata and provenance; the existing backup header is insufficient.

### Bounds valid in either build mode

The [CLI validation branches](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/cli.c)
use current table dimensions with table support and fixed fallback bounds without
it. Use their intersection to avoid guessing which build mode applies:

| Selector | Zero | Nonzero prerequisite and limit |
| --- | --- | --- |
| Band | Accepted independently of dimensions | Explicit current bands; at most `min(bands, 5)` |
| Channel | Accepted independently of dimensions | Explicit current channels; at most `min(channels, 8)` |
| Power | Accepted independently of dimensions | Explicit current powerlevels; at most `min(powerlevels, 5)` |

The fallback bounds come from the reviewed
[table header](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/drivers/vtx_table.h).
This intersection is a conservative implementation decision, not an upstream
restriction on table-enabled builds. A band 6 declaration can be valid upstream
but remains unknown here. Missing dimensions only prevent certification of their
nonzero selectors. All-zero selectors need no table dimensions. AUX indices are
0–13. All prerequisites apply at the declaration's position in command order.

Track dimension declarations independently of their dependent band/power content,
using the same invalidation policy as explicit table comparison. A reset or an
unverified table mutation clears dimension knowledge. Later dimensions do not
retroactively certify an earlier activation. Table changes do not rewrite stored
activation slots: once accepted, a slot's selector numbers remain known even if
later dimensions shrink. Equal selectors still say nothing about the selected
frequency or device power.

### Implemented scope

- Compare final explicit declarations by slot 0–9. Accept exactly seven unsigned
  decimal operands: slot, AUX, band, channel, power, start, end. Bound range input
  to 0–65535 to avoid emulating integer overflow; clamp to 900–2100 and round down
  to 25 µs steps as the CLI does. Normalize numeric spelling. Preserve source lines.
- Keep equal or reversed ranges as declared; do not silently treat them as omitted
  or equate different inactive slots. Render zero selectors as “Leave unchanged”.
- Bare `vtx` is a query. Slot-only input is an unverified mutation, not a query.
  Defaults clear slot and dimension knowledge. Conservatively clear all slot
  knowledge on malformed/unverified activation mutations, including a missing
  prerequisite; later complete declarations can restore individual slots. Do not
  invent a cleared slot from the upstream partial-assignment reset behavior.
- Compare all six stored fields after normalization. Missing/invalidated slots
  are unknown; firmware-gate failures on present declarations are not comparable.
  Preserve raw collection text even when semantic knowledge is cleared.
- Follow existing filters, hide-equal controls and source expanders. Explain that
  comparison covers declarations and that build-dependent cases remain unknown.
  Do not merge overlapping slots or assert which one wins: the reviewed runtime
  uses receiver state, slot iteration, previous selection and arming state.
- Test both build bounds, unknown dimensions, each zero-selector exception,
  declaration order, subsequent dimension changes, resets, malformed and partial
  assignments, duplicates, slot/AUX boundaries, range normalization, reversed
  ranges, overlapping slots and firmware gates. Include actual Rust parser output
  in browser coverage. Keep core diagnostics and export outside this unit.

This decision resolves a usable initial scope. Build-specific table selectors
above the intersection and all 2025.12 capacity certification remain deferred.
