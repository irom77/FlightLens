"""Developer-only voltage calibration evidence verification; no backup inputs."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import urllib.request

from verify_battery_parameter_equivalence import VERSIONS, block

CHECKS = {'cli/settings.c': ['{ "vbat_scale", VAR_UINT8 | HARDWARE_VALUE, .config.minmaxUnsigned = { '
                    'VBAT_SCALE_MIN, VBAT_SCALE_MAX }, PG_VOLTAGE_SENSOR_ADC_CONFIG, '
                    'offsetof(voltageSensorADCConfig_t, vbatscale) },',
                    '{ "vbat_divider", VAR_UINT8 | MASTER_VALUE, .config.minmaxUnsigned = { '
                    'VBAT_DIVIDER_MIN, VBAT_DIVIDER_MAX }, PG_VOLTAGE_SENSOR_ADC_CONFIG, '
                    'offsetof(voltageSensorADCConfig_t, vbatresdivval) },',
                    '{ "vbat_multiplier", VAR_UINT8 | MASTER_VALUE, .config.minmaxUnsigned = { '
                    'VBAT_MULTIPLIER_MIN, VBAT_MULTIPLIER_MAX }, PG_VOLTAGE_SENSOR_ADC_CONFIG, '
                    'offsetof(voltageSensorADCConfig_t, vbatresdivmultiplier) },'],
 'sensors/voltage.h': ['#define VBAT_SCALE_MIN 0',
                       '#define VBAT_SCALE_MAX 255',
                       '#define VBAT_DIVIDER_MIN 1',
                       '#define VBAT_DIVIDER_MAX 255',
                       '#define VBAT_MULTIPLIER_MIN 1',
                       '#define VBAT_MULTIPLIER_MAX 255',
                       'uint8_t vbatscale; // adjust this to match battery voltage to reported '
                       'value',
                       'uint8_t vbatresdivval; // resistor divider R2 (default NAZE 10(K))',
                       'uint8_t vbatresdivmultiplier; // multiplier for scale (e.g. 2.5:1 ratio '
                       "with multiplier of 4 can use '100' instead of '25' in ratio) to get better "
                       'precision'],
 'sensors/voltage.c': []}
EXPECTED_CONVERSION = "11072d9896d577a19394c4387f69e6f9976c125513ba789b8ea345e1fe40ff4d"


def verify(item):
    version, path = item
    url = f"https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}"
    data = urllib.request.urlopen(url, timeout=30).read()
    source = data.decode()
    lines = [" ".join(line.split()) for line in source.splitlines()]
    for expected in CHECKS[path]:
        if expected not in lines:
            raise ValueError(f"Review required: {version} {path}: {expected}")
    sections = {}
    if path == "sensors/voltage.c":
        prefix = "STATIC_UNIT_TESTED uint16_t voltageAdcToVoltage("
        digest = hashlib.sha256(block(source, prefix).encode()).hexdigest()
        if digest != EXPECTED_CONVERSION:
            raise ValueError(f"Review required: {version} {path}: {prefix}")
        sections[prefix] = digest
    return dict(version=version, path=path, url=url,
                sha256=hashlib.sha256(data).hexdigest(), sections=sections)


def main():
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        sources = list(pool.map(verify, [(v, p) for v in VERSIONS for p in CHECKS]))
    target = Path(__file__).resolve().parents[1] / "docs/voltage-calibration-source-evidence.json"
    target.write_text(json.dumps({"sources": sources}, indent=2) + "\n")
    print(f"Verified voltage calibration evidence across {len(VERSIONS)} releases and {len(sources)} sources.")


if __name__ == "__main__":
    main()
