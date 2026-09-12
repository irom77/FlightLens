# Battery capacity and cell-count comparison evidence

FlightLens certifies the global integer settings `bat_capacity` and
`force_battery_cell_count` across official Betaflight 4.5.0–4.5.5 and
2025.12.1–2025.12.5, subject to the existing exact-version and compatibility-pack
gates. These mappings compare configured quantities. They do not certify battery
measurement accuracy, detection timing, alarm behavior, or remaining flight time.

The capacity CLI name is **`bat_capacity`**, not `battery_capacity`.

| CLI setting | Stored field | CLI bounds | Meaning |
| --- | --- | --- | --- |
| `bat_capacity` | `batteryConfig_t.batteryCapacity`, uint16 | 0–20000 | Configured capacity in mAh; zero selects the voltage-based remaining-percentage fallback |
| `force_battery_cell_count` | `batteryConfig_t.forceBatteryCellCount`, uint8 | 0–24 | Configured cell-count override; zero leaves automatic cell counting selected |

Both declarations are global (`MASTER_VALUE`) in `PG_BATTERY_CONFIG`; neither
needs numeric conversion. The storage integer limits are broader than the CLI
bounds and must not replace them. The declarations and field documentation agree
in [4.5.0 settings](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/cli/settings.c),
[2025.12.1 settings](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/cli/settings.c),
[4.5.0 battery header](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/battery.h),
and [2025.12.1 battery header](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/battery.h).

## Runtime evidence and limits

`calculateBatteryPercentageRemaining` uses positive configured capacity together
with measured mAh consumption to calculate a constrained percentage. With zero
capacity it instead estimates percentage from voltage and configured cell-voltage
limits. With no detected cells it returns zero. `batteryUpdateConsumptionState`
requires consumption alerts enabled, positive capacity, and detected cells before
updating consumption warnings. Capacity therefore describes the configured mAh
quantity, not a promise that a current meter exists or consumption alarms run.
These calculations agree in the [4.5.0 implementation](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/sensors/battery.c)
and [2025.12.1 implementation](https://github.com/betaflight/betaflight/blob/2025.12.1/src/main/sensors/battery.c).

`batteryUpdatePresence` applies a nonzero forced cell count only after the battery
presence and voltage-stability checks succeed. Zero selects the automatic branch,
which derives cell count from voltage and caps it at eight. That automatic cap does
not constrain the forced setting's CLI range of 0–24. The resulting cell count
feeds voltage thresholds; other settings and measurements remain relevant.
The automatic branch also contains profile selection while disarmed. Certifying
the override value does not certify the selected profile or detected cell count.
See the same battery implementations and headers linked above.

The surrounding presence logic is not identical across these releases. Betaflight
4.5.1 changes the voltage-stability gate to `voltageIsStable`, and the 2025.12
baseline changes a disconnect reset literal to `0.0f`. The forced-count assignment
and automatic-count branch remain equivalent for these configured quantities.
The [4.5.1 implementation](https://github.com/betaflight/betaflight/blob/4.5.1/src/main/sensors/battery.c)
provides the intermediate comparison point. No certification of detection timing
or complete runtime behavior follows from this review.

## Release coverage and reproducibility

Research fetched the three source files above at each of all eleven exact tags
and checked normalized CLI declarations and field documentation. Both settings
match at every tag. The complete normalized
`calculateBatteryPercentageRemaining` and `batteryUpdateConsumptionState`
functions also match across all eleven tags. Full presence-function differences
were reviewed at 4.5.0, 4.5.1, and 2025.12.1; each later release matches the
corresponding latter baseline for that function.

Run `python3 -B tools/verify_battery_parameter_equivalence.py` to verify all
33 public source files. The verifier pins the CLI declarations, header fields,
and complete normalized `calculateBatteryPercentageRemaining` and
`batteryUpdatePresence` functions, accepting the reviewed presence variants at
4.5.0, 4.5.1, and 2025.12.1. The per-release hashes and source URLs are recorded in
[battery-parameter-source-evidence.json](battery-parameter-source-evidence.json).
The consumption-state function was separately reviewed across all eleven releases;
it is not pinned by this verifier. No backup input is read or transmitted.

The mappings are implemented in `src/parameterEquivalence.json`. The generator
`tools/build_parameter_equivalence.py` runs the reviewed battery verification
before writing the manifest and includes its source evidence. Unit tests cover
release identities, bounds, zero values, invalid values, scopes and packs; a
Rust-parser browser fixture checks changed capacity and equal automatic cell
count. Application comparison stays offline.

Retain existing missing/invalid-value handling and provenance. Do not introduce
defaults for absent values. Vendor suffixes, unlisted releases, other scopes, and
mismatched compatibility packs remain uncertified. Tests should cover both bounds,
zero values, changed values, and rejection of values outside the CLI ranges.
