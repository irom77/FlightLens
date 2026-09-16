"""Record pinned 4.5 PID reset evidence; this does not certify runtime defaults.

Developer-only public-source downloads. No backup data is read.
Run: python3 tools/verify_pid_defaults.py
"""
import argparse
import concurrent.futures
import hashlib
import json
import pathlib
import re
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
RELEASES = {
    "4.5.0": "c155f5830d0ffdee1c34071dd21f174ffc374c81",
    "4.5.1": "77d01ba3b76a22909d5f09cb0628820141f95eaa",
    "4.5.2": "024f8e13d4e642eb6a380308685b9ea3aa3ef1a2",
    "4.5.3": "0e533ba76cb9129458578f7fa2134df1449596ba",
    "4.5.4": "25356b59a96dc975d777844f896b7c7855ea90e6",
    "4.5.5": "4adbd3ef7cb546947600e5f747bd5453c9573063",
}
FUNCTIONS = {
    "pg/pg.c": ["pgOffset", "pgResetInstance", "pgReset", "pgResetAll"],
    "flight/pid.c": ["resetPidProfile", "pgResetFn_pidProfiles"],
    "config/config.c": ["resetConfig"],
    "config/simplified_tuning.c": ["calculateNewPidValues", "applySimplifiedTuningPids",
                                   "disableSimplifiedTuning"],
    "cli/cli.c": ["cliDefaults", "applySimplifiedTuningAllProfiles",
                  "backupPgConfig", "backupConfigs", "backupAndResetConfigs",
                  "dumpPgValue", "dumpAllValues", "printConfig", "cliDiff",
                  "cliSimplifiedTuning", "printVersion"],
}
PATHS = [*FUNCTIONS, "flight/pid.h", "config/simplified_tuning.h",
         "cli/settings.c", "target/common_pre.h", "target/common_post.h",
         "platform.h", "target/common_defaults_post.h", "common/maths.h", "common/axis.h", "Makefile", "mk/config.mk",
         "fc/core.h", "common/time.h", "target/SITL/platform_mcu.h",
         "target/SITL/target.h", "common/utils.h", "build/version.h", "pg/pg.h",
         "build/build_config.h", "sensors/battery.h", "common/filter.h",
         "sensors/current.h", "sensors/current_ids.h", "sensors/voltage.h",
         "sensors/voltage_ids.h", "config/config_reset.h", "pg/pg_ids.h"]


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def function(text, name):
    # These reviewed upstream functions end with an unindented closing brace.
    matches = re.findall(
        rf"^(?:static )?(?:void |const char \*|uint8_t \*){name}\([^\n]*\)\n\{{\n.*?^\}}", text, re.M | re.S
    )
    assert len(matches) == 1, name
    return matches[0]


def fetch(item):
    version, path = item
    relative = path if path == "Makefile" or path.startswith("mk/") else f"src/main/{path}"
    url = f"https://raw.githubusercontent.com/betaflight/betaflight/{RELEASES[version]}/{relative}"
    with urllib.request.urlopen(url, timeout=30) as response:
        return item, response.read().decode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare with committed evidence without rewriting it")
    args = parser.parse_args()
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        sources = dict(pool.map(fetch, [(v, p) for v in RELEASES for p in PATHS]))
    results = []
    for version, commit in RELEASES.items():
        macros = {}
        for path in ["flight/pid.h", "config/simplified_tuning.h"]:
            for name, value in re.findall(
                r"^#define\s+((?:PID_(?:ROLL|PITCH|YAW)|D_MIN|SIMPLIFIED_TUNING(?:_D)?)_DEFAULT)\s+([^\n]+)",
                sources[version, path], re.M,
            ):
                macros[name] = value.strip()
        assert len(macros) == 6, (version, macros)
        mappings = [line.strip() for line in sources[version, "cli/settings.c"].splitlines()
                    if re.search(r"offsetof\(pidProfile_t, pid\[PID_(ROLL|PITCH|YAW)\]\.[PIDF]\)", line)]
        assert len(mappings) == 12, version
        functions = {name: digest(function(sources[version, path], name))
                     for path, names in FUNCTIONS.items() for name in names}
        results.append({
            "version": version, "commit": commit,
            "sourceSha256": {path: digest(sources[version, path]) for path in PATHS},
            "functionSha256": functions, "macros": macros,
            "cliMappings": mappings,
            "certifiedForRecovery": False,
        })
    baseline = results[0]
    for result in results:
        result["changedFunctionsFrom450"] = [name for name, sha in result["functionSha256"].items()
                                              if sha != baseline["functionSha256"][name]]
        assert result["macros"] == baseline["macros"], result["version"]
        assert result["cliMappings"] == baseline["cliMappings"], result["version"]
    output = ROOT / "fixtures/pid-default-evidence.json"
    rendered = json.dumps(results, indent=2) + "\n"
    if args.check:
        if output.read_text() != rendered:
            raise SystemExit("PID evidence differs; review upstream evidence before regenerating")
    else:
        output.write_text(rendered)
    print(f"{len(results)} pinned releases: PID macros and 12 CLI mappings match; recovery remains uncertified")
    for result in results:
        print(result["version"], "changed functions:", ", ".join(result["changedFunctionsFrom450"]) or "none")


if __name__ == "__main__":
    main()
