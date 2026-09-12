"""Developer-only virtual-current evidence verification; no backup inputs."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import urllib.request

from verify_battery_parameter_equivalence import VERSIONS, block

CHECKS = {
    "cli/settings.c": [
        '{ "ibatv_scale", VAR_INT16 | MASTER_VALUE, .config.minmax = { -16000, 16000 }, PG_CURRENT_SENSOR_VIRTUAL_CONFIG, offsetof(currentSensorVirtualConfig_t, scale) },',
        '{ "ibatv_offset", VAR_UINT16 | MASTER_VALUE, .config.minmaxUnsigned = { 0, 16000 }, PG_CURRENT_SENSOR_VIRTUAL_CONFIG, offsetof(currentSensorVirtualConfig_t, offset) },',
    ],
    "sensors/current.h": [
        "int16_t scale; // scale the throttle to centiamperes, using a hardcoded thrust linearization function (see Battery.md)",
        "uint16_t offset; // offset of the current sensor in centiamperes (1/100th A)",
    ],
    "sensors/current.c": [],
}
EXPECTED_BLOCKS = {
    "void currentMeterVirtualRefresh(": "ebb91b83a9f7d3d87c0ec2dce268565c3f992d630ddd0130490f586aedc53bbd",
    "static void updateCurrentmAhDrawnState(": "7750bc36f50be75202ea950b76caae4d9df9ec7da3557288224646d90150dbd9",
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
        for prefix, expected in EXPECTED_BLOCKS.items():
            digest = hashlib.sha256(block(source, prefix).encode()).hexdigest()
            if digest != expected:
                raise ValueError(f"Review required: {version} {path}: {prefix}")
            sections[prefix] = digest
    return dict(version=version, path=path, url=url,
                sha256=hashlib.sha256(data).hexdigest(), sections=sections)


def main():
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        sources = list(pool.map(verify, [(v, p) for v in VERSIONS for p in CHECKS]))
    target = Path(__file__).resolve().parents[1] / "docs/virtual-current-source-evidence.json"
    target.write_text(json.dumps({"sources": sources}, indent=2) + "\n")
    print(f"Verified virtual-current evidence across {len(VERSIONS)} releases and {len(sources)} sources.")


if __name__ == "__main__":
    main()
