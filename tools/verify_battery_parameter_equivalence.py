"""Developer-only verification of candidate battery parameter mappings; no backup input."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import urllib.request

VERSIONS = [f"4.5.{n}" for n in range(6)] + [f"2025.12.{n}" for n in range(1, 6)]
CHECKS = {
    "cli/settings.c": [
        '{ "bat_capacity", VAR_UINT16 | MASTER_VALUE, .config.minmaxUnsigned = { 0, 20000 }, PG_BATTERY_CONFIG, offsetof(batteryConfig_t, batteryCapacity) },',
        '{ "force_battery_cell_count", VAR_UINT8 | MASTER_VALUE, .config.minmaxUnsigned = { 0, 24 }, PG_BATTERY_CONFIG, offsetof(batteryConfig_t, forceBatteryCellCount) },',
    ],
    "sensors/battery.h": [
        "uint16_t batteryCapacity; // mAh",
        "uint8_t forceBatteryCellCount; // Number of cells in battery, used for overwriting auto-detected cell count if someone has issues with it.",
    ],
    "sensors/battery.c": [],
}
# Reviewed complete runtime functions, including the 4.5.1 presence-gate change.
EXPECTED = {
    "4.5.0": {
        "void batteryUpdatePresence(": "c17f395271dff5a0e46b9b5c7d459169d54f63a25ee113f0153c9b719303ac3b",
        "uint8_t calculateBatteryPercentageRemaining(": "75c137fa65b5f9e99f0106fc4e56fe9c17c4347115e1dd7c1a5b1a5d66932f87"
    },
    "2025.12.1": {
        "void batteryUpdatePresence(": "49f78068e150c036788f20d8ca0b0af8d8b249a5dc3184c496870c00167480a0",
        "uint8_t calculateBatteryPercentageRemaining(": "75c137fa65b5f9e99f0106fc4e56fe9c17c4347115e1dd7c1a5b1a5d66932f87"
    },
    "4.5.1": {
        "void batteryUpdatePresence(": "6d23b6795673a7c5d2324d766acb619e536aa5541bd353a44e5200639b2be602",
        "uint8_t calculateBatteryPercentageRemaining(": "75c137fa65b5f9e99f0106fc4e56fe9c17c4347115e1dd7c1a5b1a5d66932f87"
    }
}

def block(source, prefix):
    start = source.index(prefix)
    opening = source.index("{", start)
    depth, end = 1, opening + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return " ".join(source[start:end].split())

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
    if path == "sensors/battery.c":
        baseline = ("4.5.0" if version == "4.5.0" else "4.5.1") if version.startswith("4.5.") else "2025.12.1"
        for prefix, expected in EXPECTED[baseline].items():
            digest = hashlib.sha256(block(source, prefix).encode()).hexdigest()
            if digest != expected:
                raise ValueError(f"Review required: {version} {path}: {prefix}")
            sections[prefix] = digest
    return dict(version=version, path=path, url=url,
                sha256=hashlib.sha256(data).hexdigest(), sections=sections)

def main():
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        sources = list(pool.map(verify, [(v, p) for v in VERSIONS for p in CHECKS]))
    target = Path(__file__).resolve().parents[1] / "docs/battery-parameter-source-evidence.json"
    target.write_text(json.dumps({"sources": sources}, indent=2) + "\n")
    print(f"Verified candidate battery mappings across {len(VERSIONS)} releases and {len(sources)} sources.")

if __name__ == "__main__":
    main()
