# ADC current calibration comparison evidence

FlightLens certifies `ibata_offset` as a cross-version mapping for configured integer
values across official Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5. Its CLI
declaration, storage, units, and conversion role agree across all eleven reviewed
tags. `ibata_scale` also has stable source semantics, but remains deferred because
it is hardware-specific and absent from FlightLens's current compatibility packs.
The offset mapping is implemented; the scale mapping remains deferred.

| Setting | CLI category | Stored signed 16-bit field | CLI bounds | Unit |
| --- | --- | --- | --- | --- |
| `ibata_offset` | `MASTER_VALUE` | `currentSensorADCConfig_t.offset` | −32000–32000 | mA |
| `ibata_scale` | `HARDWARE_VALUE` | `currentSensorADCConfig_t.scale` | −16000–16000 | mV per 10 A |

Both declarations target `PG_CURRENT_SENSOR_ADC_CONFIG` through direct field
offsets. The configuration is a single declared structure, with no profile or
per-sensor index. `MASTER_VALUE` supports FlightLens's global scope for the offset;
it does not make the separately classified scale a supported global parameter.
See [4.5.0 CLI declarations](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/settings.c),
[2025.12.1 CLI declarations](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/cli/settings.c),
[4.5.0 storage and units](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/current.h),
and [2025.12.1 storage and units](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/current.h).

## Conversion semantics and limits

The complete `currentMeterADCToCentiamps` function is identical after whitespace
normalization across all eleven releases. It converts the ADC sample to integer
millivolts using the measured reference voltage and a divisor of 4096. For nonzero
scale, it multiplies millivolts by 10000, divides by the signed scale, adds the
configured offset in milliamps, then divides by ten to return centiamps. Integer
division and its truncation are part of this calculation. Positive offset adds
current; negative offset subtracts it. Zero offset contributes nothing to this
addition. Zero scale returns zero from this conversion regardless of offset; it
must not be interpreted as automatic calibration. See
[4.5.0 conversion and refresh](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/current.c)
and [2025.12.1 conversion and refresh](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/current.c).

`currentMeterADCRefresh` applies the conversion to both the raw current-channel
sample and a filtered sample. Its complete normalized body is identical within
each release family. Between families the only change is the sample accessor:
`adcGetChannel(ADC_CURRENT)` becomes `adcGetValue(ADC_CURRENT)`. The no-ADC branch
sets both current readings to zero. This review verifies the conversion role and
records that accessor change; it does not certify ADC acquisition, reference
voltage, filtering, hardware wiring, or equivalent measured current.

An equal configured offset therefore means equal stored correction in mA. It does
not establish that ADC current measurement is selected or available, that scale
is nonzero, or that two boards report the same current. The virtual-current and
ESC-current paths in the same sources are separate. Do not substitute their
offset settings or units, synthesize a default for an omitted setting, or combine
offset and scale into a derived calibration comparison.

## Release coverage and reproducibility

Research fetched `src/main/cli/settings.c`, `src/main/sensors/current.h`, and
`src/main/sensors/current.c` for every exact tag: 4.5.0, 4.5.1, 4.5.2, 4.5.3,
4.5.4, 4.5.5, 2025.12.1, 2025.12.2, 2025.12.3, 2025.12.4, and 2025.12.5.
The two declarations, ADC configuration field lines, and complete conversion
function match after whitespace normalization. The refresh function has the two
reviewed family variants described above.

Run `python3 -B tools/verify_current_calibration_equivalence.py` to recheck the
33 public source files. The verifier pins those declarations, field lines,
conversion, and refresh bodies. Per-release source URLs and hashes are recorded
in [current-calibration-source-evidence.json](current-calibration-source-evidence.json).
This developer verification needs no backup contents and adds no application
network access.

## Implemented scope

Of these two candidates, the manifest includes `ibata_offset` as a global signed
integer mapping bounded to −32000–32000, with mA units. Exact official-release,
family, and compatibility-pack gates still apply. Missing or invalid values remain
unknown; uncertified identities or scopes remain not comparable. Tests cover
negative values, zero, both bounds, out-of-range values, reversed comparisons,
and unsupported identities.

Both bundled compatibility packs already type `ibata_offset` as a global integer.
The 4.5.0 pack has null numeric bounds; the 2025.12.1 pack declares −32000–32000.
The mapping enforces the independently verified limits even when
the underlying pack does not. Both packs omit `ibata_scale`; support for that
setting needs a separate hardware-schema and typing decision before this
research can enable a mapping.

`tools/build_parameter_equivalence.py` verifies this evidence before generating
`src/parameterEquivalence.json` and bundles it in `currentSources`. Unit coverage
checks all eleven releases, negative values, zero, bounds, and compatibility gates.
A Rust-parser browser fixture compares offsets of −32000 and 32000. Application
comparison remains offline and preserves existing provenance.
