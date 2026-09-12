# Voltage calibration comparison evidence

The global integer settings `vbat_divider` and `vbat_multiplier` are certified for configured-value comparison across official Betaflight 4.5.0–4.5.5
and 2025.12.1–2025.12.5. Their declarations, bounds, storage, and conversion roles
agree in every reviewed release. `vbat_scale` has the same conversion role across
these releases, but remains deferred because it is hardware-specific and absent
from FlightLens's current compatibility packs for these families. The two divisor mappings are implemented with exact release and pack gates.

| Setting | Declaration category | Stored uint8 field | CLI bounds | Conversion role |
| --- | --- | --- | --- | --- |
| `vbat_scale` | `HARDWARE_VALUE` | `vbatscale` | 0–255 | Multiplies the ADC reading before division |
| `vbat_divider` | `MASTER_VALUE` | `vbatresdivval` | 1–255 | Divides the scaled ADC reading |
| `vbat_multiplier` | `MASTER_VALUE` | `vbatresdivmultiplier` | 1–255 | Applies a further divisor despite its CLI name |

All three declarations target `PG_VOLTAGE_SENSOR_ADC_CONFIG` and fields of
`voltageSensorADCConfig_t`. The header declares an array of these configurations;
the CLI declarations have direct field offsets and no per-sensor index operand.
See [4.5.0 CLI declarations](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/settings.c),
[2025.12.1 CLI declarations](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/cli/settings.c),
[4.5.0 bounds and storage](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/voltage.h),
and [2025.12.1 bounds and storage](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/voltage.h).

## Conversion semantics and limits

The complete `voltageAdcToVoltage` function is identical after whitespace
normalization across all eleven tags. It multiplies the ADC sample by the scale
and measured reference voltage, divides by ten, adds a rounding term, divides by
4095 times the configured divider, then divides by the configured multiplier.
Its integer result represents voltage in 0.01 V steps. The divider and multiplier
are therefore stored divisor factors, not quantities measured in volts. Zero is
outside both divisor settings' accepted bounds. Zero scale is accepted, but this
research does not assign it an undocumented disable or automatic meaning.
See [4.5.0 conversion](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/voltage.c)
and [2025.12.1 conversion](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/voltage.c).

The adjacent `voltageMeterADCRefresh` function obtains a configuration for each
ADC sensor index and applies this conversion to raw and filtered ADC samples.
Its channel map starts with `ADC_BATTERY`; additional rail channels depend on
build definitions. The refresh function is identical within each reviewed
release family. Between families, the sample accessor changes from
`adcGetChannel` to `adcGetValue`, and the no-ADC branch adds
`UNUSED(voltageMeterAdcChannelMap)`. These differences do not alter the conversion
formula, but this review does not certify equivalent ADC acquisition, reference
voltage, hardware wiring, filtering, or measured voltage. Source links above
contain the channel map and complete refresh functions.

Compare each explicitly configured integer independently. Do not collapse
multiple settings into an effective gain: sequential integer divisions and
rounding are part of the formula. Equal values describe equal configured factors;
they do not establish equivalent measurements on different boards or builds.
Do not infer ADC availability or that ADC is the selected battery meter from the
presence of these settings. ESC voltage handling is a separate path in the same
source files.

## Release coverage and reproducibility

Research fetched `src/main/cli/settings.c`, `src/main/sensors/voltage.h`, and
`src/main/sensors/voltage.c` at all eleven exact tags. The three normalized CLI
declarations, six bound definitions, three storage field lines, and complete
normalized conversion function match across every tag. The refresh function and
channel map were reviewed separately; they are not pinned by the verifier.

Run `python3 -B tools/verify_voltage_calibration_equivalence.py` to recheck the
33 public source files. Pinned hashes and per-release source URLs are recorded in
[voltage-calibration-source-evidence.json](voltage-calibration-source-evidence.json).
No backup contents are needed or transmitted. This developer verification does
not introduce network access into FlightLens.

## Implementation

The cross-version manifest includes `vbat_divider` and `vbat_multiplier` as global
integers bounded to 1–255, labeled as factors. The generator verifies the reviewed
source blocks before emitting mappings and includes the evidence in the manifest.

Preserve exact official release, firmware-family, and compatibility-pack gates;
missing or invalid values remain unknown, and uncertified identities or scopes
remain not comparable. Do not introduce defaults for omitted settings. Unit tests cover both
bounds, rejected zero and out-of-range values, equal and changed values, reversed
comparison direction, all eleven releases, and unsupported scopes, packs, and
vendor suffixes. A Rust-parser browser fixture checks changed divider and equal multiplier values.

FlightLens's `betaflight-4.5.0.json` and `betaflight-2025.12.1.json` compatibility
packs include the two divisor settings as global integers but omit `vbat_scale`.
The former pack does not currently attach numeric bounds to these divisor
settings; the cross-version mapping must enforce the independently verified
bounds itself. Do not expand hardware-specific schema support solely to map
`vbat_scale`. Its future support needs an explicit decision about hardware
settings, parser typing, and compatibility before this evidence can enable it.
