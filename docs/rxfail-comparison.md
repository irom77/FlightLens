# Receiver failsafe comparison: verified scope

Source verification and semantic comparison are complete locally.
The existing collection view continues to compare source text.

Run `python3 tools/verify_rxfail_comparison.py` to verify the pinned reviewed
sections and regenerate [source evidence](rxfail-source-evidence.json).
All 11 official releases (4.5.0–4.5.5 and 2025.12.1–2025.12.5) have identical
reviewed CLI handlers, dump formatting, mode tables, conversion constants,
enumerations, and runtime value selection. Evidence records 33 source URLs,
whole-file hashes, and normalized section hashes. The developer tool uses the
network; the application remains offline.

## Upstream findings

The [CLI handler and mode table](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/cli.c)
accept auto (`a`), hold (`h`), and set (`s`) on channels 0–3; channels 4–17
accept hold or set. Set requires a value. Auto and hold reject a value.
Repeated assignments replace that channel's mode; set also replaces its stored
step. Bare `rxfail` and a channel-only invocation query existing settings.
There is no dedicated reset subcommand.

The [receiver declarations](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/rx/rx.h)
define 18 channels and a 750–2250 µs range. Set values are clamped, then
quantized down to 25 µs steps from 750. For example, 1001 and 1024 both store
1000 µs. CLI conversion first narrows the parsed integer to an unsigned
16-bit value; a conservative comparison should avoid emulating overflow and
permissive C parsing.

The [runtime value selection](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/rx/rx.c)
depends on additional settings and state for auto and hold. A stored set value
is converted back from its step. Equal declarations therefore do not establish
equal flight behavior or validate a failsafe configuration.

## Implemented comparison scope

- Gate semantic results on identical official version, lowercase `betaflight`
  family, and the expected bundled pack ID. Cross-version and vendor firmware
  remain not comparable.
- Compare final explicit declarations by channel: mode plus normalized set value.
  Ignore the hidden stored step for auto/hold because dumps omit it.
- Interpret exact lowercase modes and unsigned decimal operands only. Accept set
  inputs within 750–2250 and quantize them; treat other syntax conservatively.
- Preserve bare and valid channel-only queries. Defaults and unverified mutations
  clear prior knowledge; subsequent valid declarations restore individual channels.
  Missing declarations stay unknown; do not infer reset defaults.
- Add channel filtering, source lines, and hide-equal controls. Retain every raw
  occurrence in the existing collection view.
- Cover mode/channel restrictions, quantization boundaries, last assignment,
  queries, resets, malformed input, missing values, and firmware gates. Include
  a browser test with actual Rust parser output.

Core collection diagnostics and export remain outside this slice.
