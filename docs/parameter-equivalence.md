# Certified parameter comparison

The mappings compare global integer settings between official
Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5 releases, including pairs within
those ranges. They certify these individual quantities, not overall flight behavior:

| Setting | Meaning | Bounds |
| --- | --- | --- |
| `ibatv_scale` | Configured virtual-current coefficient | −16000–16000 |
| `ibatv_offset` | Configured virtual-current offset in 0.01 A | 0–16000 |
| `ibata_offset` | Configured ADC current correction in mA | −32000–32000 |
| `motor_poles` | Magnetic pole count | 4–255 |
| `bat_capacity` | Configured capacity in mAh; zero selects voltage-based percentage | 0–20000 |
| `vbat_divider` | Configured voltage divisor factor | 1–255 |
| `vbat_multiplier` | Configured additional voltage divisor factor | 1–255 |
| `force_battery_cell_count` | Configured cell count override; zero selects automatic counting | 0–24 |
| `vbat_max_cell_voltage` | Maximum cell voltage for automatic cell-count detection | 100–500 |
| `vbat_min_cell_voltage` | Cell voltage for the critical alarm | 100–500 |
| `vbat_warning_cell_voltage` | Cell voltage for the warning alarm | 100–500 |

The three cell-voltage thresholds use 0.01 V units. No value conversion is needed.

`src/parameterEquivalence.json` records exact versions, expected compatibility
pack IDs, scope, type, bounds, and source URLs with SHA-256 hashes. The developer
command `python3 -B tools/build_parameter_equivalence.py` fetches only public
upstream sources and verifies the CLI declaration, CLI name, field documentation,
and electrical-RPM conversion or battery threshold calculation at each tag before regenerating the manifest.
This command is never run by the application; comparison remains offline.

The evidence is the matching global uint8 declaration (4–255), magnetic-pole
field definition, and conversion dividing by half the pole count in
`cli/settings.c`, `fc/parameter_names.h`, `pg/motor.h`, and `drivers/dshot.c`.
Battery evidence includes the matching global uint16 declarations, 100–500
range constants, field documentation specifying 0.01 V, and the cell-count and
alarm-threshold calculations in `sensors/battery.h` and `sensors/battery.c`.
Exact source links are bundled per release. Matching setting names or schemas
alone are insufficient evidence for adding another mapping.

Different versions use these mappings only when both exact release identities and
pack IDs are certified. Vendor suffixes, prereleases, future patches, other
families, scopes, and settings do not inherit it. Invalid or missing values stay
Unknown. Existing same-version comparisons retain their behavior. No defaults
are introduced; declared/derived provenance remains visible. CLI collections
and further parameter mappings remain separate work.


Capacity and cell-count mappings also verify full battery percentage and presence
functions through `tools/verify_battery_parameter_equivalence.py` before manifest
generation. The manifest's `batterySources` records this evidence. Reviewed
presence-function differences are pinned per release baseline; these mappings
certify configured quantities, not detection timing or complete runtime behavior.
See [battery evidence and zero semantics](battery-parameter-equivalence.md).

Voltage divisor mappings also run `tools/verify_voltage_calibration_equivalence.py`
verification through the generator and bundle the reviewed source evidence. See
[voltage calibration evidence](voltage-calibration-equivalence.md) for scope and hardware limitations.

ADC current offset mapping also runs `tools/verify_current_calibration_equivalence.py`
through the generator, bundling its evidence in `currentSources`. See
[current calibration evidence](current-calibration-equivalence.md) for signed units
and the limits of comparing configured corrections rather than measured current.

Virtual-current mappings run `tools/verify_virtual_current_equivalence.py` through
the generator and bundle evidence in `virtualSources`. See
[virtual-current evidence](virtual-current-equivalence.md) for units and limits:
configured values do not certify estimated current, build support or activation.
