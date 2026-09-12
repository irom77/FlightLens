"""Developer-only verification of the certified parameter comparison mappings."""
import hashlib
import json
from pathlib import Path
import urllib.request
from verify_battery_parameter_equivalence import CHECKS as BATTERY_CHECKS, verify as verify_battery

from verify_voltage_calibration_equivalence import CHECKS as VOLTAGE_CHECKS, verify as verify_voltage

from verify_current_calibration_equivalence import CHECKS as CURRENT_CHECKS, verify as verify_current

from verify_virtual_current_equivalence import CHECKS as VIRTUAL_CHECKS, verify as verify_virtual

ROOT = Path(__file__).resolve().parents[1]
releases = {**{f'4.5.{n}': 'betaflight-4.5.0-schema-1' for n in range(6)},
            **{f'2025.12.{n}': 'betaflight-2025.12.1-schema-1' for n in range(1, 6)}}
checks = {
    'cli/settings.c': '{ PARAM_NAME_MOTOR_POLES, VAR_UINT8 | MASTER_VALUE, .config.minmaxUnsigned = { 4, UINT8_MAX }, PG_MOTOR_CONFIG, offsetof(motorConfig_t, motorPoleCount) },',
    'fc/parameter_names.h': '#define PARAM_NAME_MOTOR_POLES "motor_poles"',
    'pg/motor.h': 'uint8_t motorPoleCount; // Number of magnetic poles in the motor bell for calculating actual RPM from eRPM provided by ESC telemetry',
    'drivers/dshot.c': 'erpmToHz = ERPM_PER_LSB / SECONDS_PER_MINUTE / (motorConfig()->motorPoleCount / 2.0f);',
}
battery_fields = {
    'max': ('maximum voltage per cell, used for auto-detecting battery voltage in 0.01V units, default is 430 (4.30V)', 'unsigned cells = (voltageMeter.displayFiltered / batteryConfig()->vbatmaxcellvoltage) + 1;'),
    'min': ('minimum voltage per cell, this triggers battery critical alarm, in 0.01V units, default is 330 (3.30V)', 'batteryCriticalVoltage = batteryCellCount * batteryConfig()->vbatmincellvoltage;'),
    'warning': ('warning voltage per cell, this triggers battery warning alarm, in 0.01V units, default is 350 (3.50V)', 'batteryWarningVoltage = batteryCellCount * batteryConfig()->vbatwarningcellvoltage;'),
}
checks = {path: [line] for path, line in checks.items()}
checks['sensors/battery.h'] = [
    '#define VBAT_CELL_VOTAGE_RANGE_MIN 100',
    '#define VBAT_CELL_VOTAGE_RANGE_MAX 500',
]
checks['sensors/battery.c'] = []
mappings = [{'scope': 'global', 'key': 'motor_poles', 'kind': 'integer',
             'min': 4, 'max': 255}]
for name, (description, usage) in battery_fields.items():
    key = f'vbat_{name}_cell_voltage'
    field = f'vbat{name}cellvoltage'
    checks['cli/settings.c'].append(
        f'{{ "{key}", VAR_UINT16 | MASTER_VALUE, .config.minmaxUnsigned = {{ VBAT_CELL_VOTAGE_RANGE_MIN, VBAT_CELL_VOTAGE_RANGE_MAX }}, PG_BATTERY_CONFIG, offsetof(batteryConfig_t, {field}) }},')
    checks['sensors/battery.h'].append(f'uint16_t {field}; // {description}')
    checks['sensors/battery.c'].append(usage)
    mappings.append({'scope': 'global', 'key': key, 'kind': 'integer',
                     'min': 100, 'max': 500, 'unit': '0.01 V'})
mappings.extend([
    {'scope': 'global', 'key': 'ibatv_scale', 'kind': 'integer',
     'min': -16000, 'max': 16000, 'unit': 'coefficient'},
    {'scope': 'global', 'key': 'ibatv_offset', 'kind': 'integer',
     'min': 0, 'max': 16000, 'unit': '0.01 A'},
    {'scope': 'global', 'key': 'ibata_offset', 'kind': 'integer',
     'min': -32000, 'max': 32000, 'unit': 'mA'},
    {'scope': 'global', 'key': 'bat_capacity', 'kind': 'integer',
     'min': 0, 'max': 20000, 'unit': 'mAh'},
    {'scope': 'global', 'key': 'force_battery_cell_count', 'kind': 'integer',
     'min': 0, 'max': 24, 'unit': 'cells'},
    {'scope': 'global', 'key': 'vbat_divider', 'kind': 'integer',
     'min': 1, 'max': 255, 'unit': 'factor'},
    {'scope': 'global', 'key': 'vbat_multiplier', 'kind': 'integer',
     'min': 1, 'max': 255, 'unit': 'factor'},
])
# Verify reviewed full runtime functions before publishing the expanded manifest.
battery_sources = [verify_battery((version, path))
                   for version in releases for path in BATTERY_CHECKS]
voltage_sources = [verify_voltage((version, path))
                   for version in releases for path in VOLTAGE_CHECKS]
current_sources = [verify_current((version, path))
                   for version in releases for path in CURRENT_CHECKS]
virtual_sources = [verify_virtual((version, path))
                   for version in releases for path in VIRTUAL_CHECKS]
sources = []
for version in releases:
    for path, expected in checks.items():
        url = f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}'
        data = urllib.request.urlopen(url).read()
        lines = [' '.join(line.split()) for line in data.decode().splitlines()]
        for line in expected:
            assert line in lines, f'Review required: {version} {path}: {line}'
        sources.append({'version': version, 'path': path, 'url': url,
                        'sha256': hashlib.sha256(data).hexdigest()})
manifest = {'family': 'betaflight', 'releases': releases,
            'mappings': mappings, 'sources': sources, 'batterySources': battery_sources,
            'voltageSources': voltage_sources, 'currentSources': current_sources,
            'virtualSources': virtual_sources}
(ROOT / 'src/parameterEquivalence.json').write_text(json.dumps(manifest, indent=2) + '\n')
