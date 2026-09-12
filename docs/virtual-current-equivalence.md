# Virtual current parameter comparison evidence

FlightLens compares the bounded configured values of `ibatv_scale` and
`ibatv_offset` across official Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5.
Their CLI declarations, storage, and virtual-current calculation agree across all
eleven reviewed tags. These mappings do not certify equal current readings or consumption estimates.

| Setting | CLI category | Stored field | CLI bounds | Meaning |
| --- | --- | --- | --- | --- |
| `ibatv_scale` | `MASTER_VALUE`, signed integer | `currentSensorVirtualConfig_t.scale`, `int16_t` | −16000–16000 | Coefficient in the throttle-to-current calculation |
| `ibatv_offset` | `MASTER_VALUE`, unsigned integer | `currentSensorVirtualConfig_t.offset`, `uint16_t` | 0–16000 | Additive current in centiamperes (0.01 A) |

Both declarations use `PG_CURRENT_SENSOR_VIRTUAL_CONFIG` with direct field
offsets, and the header declares a single configuration structure. These are
global values, with no profile or array index. Unlike `ibata_scale`, virtual
scale is not classified as hardware-specific. See the
[4.5.0 declarations](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/settings.c),
[2025.12.1 declarations](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/cli/settings.c),
[4.5.0 storage](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/current.h),
and [2025.12.1 storage](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/current.h).

## Calculation and units

`currentMeterVirtualRefresh` starts its current estimate at the configured offset.
When armed, it sets the supplied throttle offset to zero if throttle is low and
motor stop is enabled, then computes an integer throttle factor:

```text
factor = throttleOffset + throttleOffset * throttleOffset / 50
current_centiamps = offset + factor * scale / 1000
```

The second term is added only when armed. Integer divisions and truncation are
part of the calculation. Scale is a coefficient for this particular formula,
not a standalone measured current or the ADC scale in mV per 10 A. Zero scale
removes the throttle contribution; negative scale reverses its sign. Zero offset
removes the additive contribution, but does not disable the virtual meter. The
virtual offset unit is **centiamperes**, whereas `ibata_offset` uses **milliamps**;
the settings are not interchangeable. See the field comments above and the
[4.5.0 calculation](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/current.c)
and [2025.12.1 calculation](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/current.c).

The refresh function passes the resulting estimate and elapsed update interval
to `updateCurrentmAhDrawnState`, which accumulates estimated consumption. Both
complete functions match after whitespace normalization in all eleven reviewed
tags. This establishes the configured values' calculation role; it does not
establish a physical current measurement or reliable battery capacity estimate.

## Runtime limits

The virtual meter implementation and its battery caller are conditional on
`USE_VIRTUAL_CURRENT_METER`. `batteryUpdateCurrentMeter` selects it only for
`CURRENT_METER_VIRTUAL`; if no battery cells are detected, the caller resets the
current meter and returns first. It passes arming state, the low-throttle and
motor-stop condition, and `lrintf(mixerGetThrottle() * 1000)` as the throttle input.
The complete normalized caller is identical across the eleven reviewed releases;
no caller variation was found in those tags. See
[4.5.0 battery caller](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/battery.c)
and [2025.12.1 battery caller](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/battery.c).

Equal configured values therefore do not establish that a firmware build includes
or selects virtual current, that battery detection agrees, or that mixer output,
arming state, motor-stop behavior, update timing, or actual power consumption
agrees. Underlying mixer and throttle implementations were not certified by this
review. Do not combine the two settings into a derived equivalent calibration,
interpret them as flight-safety advice, or fill omitted values from defaults.

## Release coverage and reproducibility

Research fetched `src/main/cli/settings.c`, `src/main/sensors/current.h`,
`src/main/sensors/current.c`, and `src/main/sensors/battery.c` for every exact tag:
4.5.0, 4.5.1, 4.5.2, 4.5.3, 4.5.4, 4.5.5, 2025.12.1, 2025.12.2, 2025.12.3,
2025.12.4, and 2025.12.5. This is 44 public source files, with no backup inputs.

Run `python3 -B tools/verify_virtual_current_equivalence.py` to recheck the
33 declaration, storage, and calculation source files. It pins the two CLI
declarations, two configuration field lines, and complete virtual-refresh and
consumption-accumulation functions. Per-release URLs and hashes are recorded in
[virtual-current-source-evidence.json](virtual-current-source-evidence.json).
The separate battery-caller review is not pinned by that verifier. These tools
are developer verification only and add no application network access.

## Implemented comparison

The two global integer mappings enforce independent bounds: `ibatv_scale` at
−16000–16000 with a coefficient label, and `ibatv_offset` at 0–16000 with a
centiampere label. They preserve exact official-release, firmware-family, and
compatibility-pack gates, as well as existing missing-value and provenance rules.
Tests cover negative scale, zero, both bounds, out-of-range values on both sides,
reversed comparisons, and unsupported identities and scopes.

Both bundled compatibility packs already type both parameters as global integers.
The [4.5.0 pack](../crates/flightlens-core/compatibility/betaflight-4.5.0.json)
has null scale bounds but explicit 0–16000 offset bounds; the
[2025.12.1 pack](../crates/flightlens-core/compatibility/betaflight-2025.12.1.json)
has the complete bounds for both. The mappings enforce the independently
verified limits even when a pack omits them. Hardware-schema work is not required
for these two virtual-current mappings.
