# Adjustment range comparison: source review and implemented scope

The `adjrange` declaration comparison UI is implemented for matching verified
official releases. Preserved collection text remains available.
Run `python3 tools/verify_adjrange_comparison.py` to regenerate
[source evidence](adjrange-source-evidence.json). It verified 55 upstream files
across official 4.5.0–4.5.5 and 2025.12.1–2025.12.5: three normalized CLI
sections and four complete supporting files per release, against pinned family
baselines. The checked sections match within each family. This developer tool
uses the network; the application remains offline. The separately linked range
activation implementation was reviewed at the 4.5.0 baseline and is not one of
the verifier's pinned files; runtime equivalence is outside the comparison scope.

## CLI declarations

The [4.5 CLI](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/cli.c)
and [2025.12 CLI](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/cli/cli.c)
provide `cliAdjustmentRange`, `printAdjustmentRange`, and the shared
`processChannelRangeArgs` helper. Declaration operands are:

`slot unused rangeAUX start end function selectAUX [center [scale]]`

Seven operands are required. The obsolete second operand is consumed but not
stored; dumps print zero there. Successful assignments reset omitted center and
scale to zero, so omission does not preserve an earlier value. Dumps always print
both optional values. Bare `adjrange` queries all slots. A slot alone attempts a
mutation. Invalid required arguments can partially assign fields before clearing
the entire addressed slot. Extra operands are ignored; integer parsing accepts
forms outside canonical dump syntax. The slot check lacks a lower-bound test.
There is no dedicated reset subcommand. Do not emulate permissive parsing or
negative indexing.

The shared range helper clamps to 900–2100 and rounds down to 25 µs steps.
[Range definitions](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc_modes.h)
explain normalization; [receiver constants](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/rx/rx.h)
limit each AUX index to 0–13. These are zero-based AUX indices, not receiver
channel numbers or proof that the receiver supplies fourteen auxiliary channels.

## Storage and release differences

The [4.5 declarations](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc_adjustments.h)
fix thirty slots (0–29). Stored fields are range AUX, normalized start/end,
adjustment function, select AUX, center and scale. Center and scale are unsigned
16-bit values; upstream CLI assignment does not explicitly validate their bounds.
Function IDs run from 0 through 34. The
[2025.12 declarations](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/fc/rc_adjustments.h)
retain fixed capacity and append LED dimmer as function 35. Unlike VTX activation
capacity, this header does not make slot count overrideable with `#ifndef`.
Source verification still does not certify vendor builds or cross-version pairs.

## Runtime limits

The [4.5 adjustment runtime](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc_adjustments.c)
uses function-specific tables, receiver values, timing, current profiles and
compiled features. Center zero selects ordinary step processing for step-mode
functions; nonzero center enables scaled absolute adjustment. Select-mode
functions use switch positions. Some effects are conditional on OSD, PID audio,
or LED support. The [2025.12 runtime](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/fc/rc_adjustments.c)
adds LED dimmer handling and changes the receiver-signal helper name.

All-zero stored records are skipped during active-range collection. Do not label
any record with function zero as safely disabled: the runtime obtains a table
entry using function minus one for nondefault records. Comparing stored fields
must not imply validation of that runtime behavior or firmware safety.

[Range activation](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc_modes.c)
requires start less than end and tests an inclusive lower/exclusive upper bound
against constrained receiver input. Equal or reversed endpoints remain stored
records. Overlapping slots must remain separate; do not predict their combined
adjustment or equate distinct inactive declarations.

## Conservative implementation

These are FlightLens design decisions, narrower than upstream CLI acceptance:

- Gate on matching official verified version, lowercase `betaflight`, and the
  expected schema pack. Include a release only after verifier coverage succeeds.
  Vendor, unsupported and cross-version comparisons remain not comparable.
- Accept exactly seven, eight or nine unsigned decimal operands. Require slot
  0–29, unused operand zero, both AUX values 0–13, and the release-specific
  function bound. Limit center, scale and range inputs to 0–65535. Reject signs,
  nondecimal suffixes, extra operands, overflow and nonzero unused values.
- Normalize leading zeros, clamp/quantize endpoints, and fill omitted center or
  scale with zero. Compare all seven stored fields by slot. Retain numeric function
  IDs as authoritative; optional labels must use the version-specific enum.
- Process commands in order: last complete explicit assignment wins. Bare queries
  preserve knowledge. Defaults clear it. An unverified or malformed mutation
  clears all slot knowledge conservatively; later complete declarations restore
  individual slots. Do not infer zero records from partial-assignment clearing.
- Missing or invalidated records are unknown. Present records failing the firmware
  gate are not comparable. Keep original occurrences and source lines accessible.
- Follow existing filter, hide-equal and source-expander controls. Describe results
  as stored declarations, not proof of identical in-flight effects. Do not infer
  build features, receiver availability, resulting gains or active profiles.

## Implementation checks

Cover minimum/maximum slots and AUX indices; family-specific function 35;
seven/eight/nine-operand forms; optional values resetting prior values; zero and
maximum unsigned storage; leading zeros; clamped and quantized endpoints;
equal/reversed ranges; explicit all-zero records; function-zero nondefault records;
nonzero unused operands; malformed/partial/extra operands; duplicate slots;
defaults; bare queries; invalidation followed by restoration; overlapping ranges;
missing records; and all firmware gates. Include a browser fixture produced by the
actual Rust parser. Core diagnostics and export remain outside this slice.


The implementation retains understood records with function IDs through 35 for
unsupported/vendor versions solely to show “Not comparable”; it does not certify
those versions. Exact official 4.5 releases use the narrower bound of 34.
Unit coverage exercises normalization, optional resets, bounds, invalidation,
source retention and firmware gates. A Rust-parser browser fixture checks equal,
changed and unknown rows, filtering, hide-equal controls and source expansion.
