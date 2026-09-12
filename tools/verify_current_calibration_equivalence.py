"""Developer-only current calibration evidence verification; no backup inputs."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import urllib.request

from verify_battery_parameter_equivalence import VERSIONS, block

CHECKS = {
    "cli/settings.c": [
        '{ "ibata_scale", VAR_INT16 | HARDWARE_VALUE, .config.minmax = { -16000, 16000 }, PG_CURRENT_SENSOR_ADC_CONFIG, offsetof(currentSensorADCConfig_t, scale) },',
        '{ "ibata_offset", VAR_INT16 | MASTER_VALUE, .config.minmax = { -32000, 32000 }, PG_CURRENT_SENSOR_ADC_CONFIG, offsetof(currentSensorADCConfig_t, offset) },',
    ],
    "sensors/current.h": [
        "int16_t scale; // scale the current sensor output voltage to milliamps. Value in mV/10A",
        "int16_t offset; // offset of the current sensor in mA",
    ],
    "sensors/current.c": [],
}
EXPECTED_CONVERSION = "a48edb61554f10d59c0d85a4d61eb37e4aeda282d499b8b41aaea5a388d647bb"
# Reviewed refresh difference: adcGetChannel becomes adcGetValue in 2025.12.
EXPECTED_REFRESH = {
    "4.5": "f2c24d34023868f27e806e105b4e60b5e120635970c45284b44b77958c7a4e36",
    "2025.12": "c23599b3dce8ca2aa9689dd3077ab6f10a8e312ae41d73a88e1b51f6304d4cdb",
}


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
    if path == "sensors/current.c":
        family = "4.5" if version.startswith("4.5.") else "2025.12"
        expected_blocks = {
            "static int32_t currentMeterADCToCentiamps(": EXPECTED_CONVERSION,
            "void currentMeterADCRefresh(": EXPECTED_REFRESH[family],
        }
        for prefix, expected in expected_blocks.items():
            digest = hashlib.sha256(block(source, prefix).encode()).hexdigest()
            if digest != expected:
                raise ValueError(f"Review required: {version} {path}: {prefix}")
            sections[prefix] = digest
    return dict(version=version, path=path, url=url,
                sha256=hashlib.sha256(data).hexdigest(), sections=sections)


def main():
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        sources = list(pool.map(verify, [(v, p) for v in VERSIONS for p in CHECKS]))
    target = Path(__file__).resolve().parents[1] / "docs/current-calibration-source-evidence.json"
    target.write_text(json.dumps({"sources": sources}, indent=2) + "\n")
    print(f"Verified current calibration evidence across {len(VERSIONS)} releases and {len(sources)} sources.")


if __name__ == "__main__":
    main()
