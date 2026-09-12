# Certified parameter comparison

The mappings compare global integer settings between official
Betaflight 4.5.0–4.5.5 and 2025.12.1–2025.12.5 releases, including pairs within
those ranges. They certify these individual quantities, not overall flight behavior:

| Setting | Meaning | Bounds |
| --- | --- | --- |
| `motor_poles` | Magnetic pole count | 4–255 |
| `vbat_max_cell_voltage` | Maximum cell voltage for automatic cell-count detection | 100–500 |
| `vbat_min_cell_voltage` | Cell voltage for the critical alarm | 100–500 |
| `vbat_warning_cell_voltage` | Cell voltage for the warning alarm | 100–500 |

All three voltage values use 0.01 V units. No value conversion is needed.

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
