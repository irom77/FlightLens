# Receiver-range comparison

FlightLens compares final explicit `rxrange` endpoints by numeric channel index
on identical verified official releases: 4.5.0–4.5.5 and 2025.12.1–2025.12.5.
The firmware family and bundled schema ID must also match the manifest.

Verification uses upstream [cliRxRange](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/cli.c)
and [receiver declarations](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/rx/rx.h).
Run `python3 tools/build_rxrange_comparison.py` to verify all 11 releases and
regenerate `src/rxrangeCompatibility.json`. The script checks the complete
normalized CLI function against its reviewed hash and the channel/bound constants;
the manifest records source URLs and SHA-256 hashes. Only this developer tool
uses the network. The application works offline.

The CLI assigns minimum and maximum independently to one of four channels,
accepting endpoints from 750 through 2250 microseconds. It does not require
minimum to precede maximum. Repeated declarations overwrite that channel.
FlightLens compares numeric endpoints, retaining the last source line.

FlightLens deliberately interprets only three unsigned decimal operands within
those bounds. `rxrange reset`, defaults resets, and unverified mutation syntax
clear earlier explicit knowledge; later valid declarations restore knowledge per
channel. It does not emulate permissive C integer parsing or infer reset defaults.
An invalid declaration therefore conservatively loses knowledge even if firmware
would reject it without changing settings. Bare read-only `rxrange` queries do
not change knowledge. Missing channels stay unknown.

Equality describes these declared endpoints only. It does not certify usable
calibration, channel mapping, receiver behavior, vendor firmware, or cross-version
equivalence. The separate collection text view retains every source occurrence,
including overwritten and pre-reset lines. Core collection diagnostics and export
remain unchanged; this comparison does not add a firmware command interpreter.
