"""Preprocess one pinned public board configuration; does not certify recovery.

Usage: python3 tools/check_pid_hardware.py FIRMWARE_SOURCE CONFIG_SOURCE [--check]
Requires a GCC-compatible preprocessor (CC, default cc). Downloads only public
upstream source files to verify compiler-discovered dependencies. Reads no backups.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import urllib.request

from verify_pid_defaults import ROOT, RELEASES

CONFIG_COMMIT = "7b1f01a25d8cb6379ebeeeca9f0e91c24925e907"
BOARD = "SPEEDYBEEF405V4"
INCLUDES = [
    "src/main", "src/main/target/STM32F405", "src/main/drivers/stm32",
    "src/main/startup", "lib/main/CMSIS/Core/Include",
    "lib/main/STM32F4/Drivers/CMSIS/Device/ST/STM32F4xx",
    "lib/main/STM32F4/Drivers/STM32F4xx_StdPeriph_Driver/inc",
]
DEFINES = ["USE_CONFIG", "STM32F40_41xxx", "STM32F405xx", "STM32", "USE_STDPERIPH_DRIVER"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("firmware", type=Path)
    parser.add_argument("config", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    roots = {"betaflight": args.firmware.resolve(), "config": args.config.resolve()}
    commits = {"betaflight": RELEASES["4.5.0"], "config": CONFIG_COMMIT}
    compiler = os.environ.get("CC", "cc")
    command = [compiler, *[f"-D{d}" for d in DEFINES], "-I", str(roots["config"] / "configs" / BOARD)]
    for include in INCLUDES:
        command += ["-I", str(roots["betaflight"] / include)]
    with tempfile.TemporaryDirectory(prefix="flightlens-hardware-") as temp:
        source = Path(temp) / "features.c"
        source.write_text('#include "platform.h"\n')
        deps = subprocess.check_output([*command, "-MM", str(source)], text=True)
        files = {Path(p).resolve() for p in shlex.split(deps.replace("\\\n", "").split(":", 1)[1])}
        files.remove(source)
        # Also pin the makefile from which the selected MCU flags/includes came.
        files.add(roots["betaflight"] / "mk/mcu/STM32F4.mk")

        def verify(path):
            matches = [(repo, path.relative_to(root).as_posix()) for repo, root in roots.items()
                       if path.is_relative_to(root)]
            assert len(matches) == 1, f"Unexpected dependency: {path}"
            repo, relative = matches[0]
            url = f"https://raw.githubusercontent.com/betaflight/{repo}/{commits[repo]}/{relative}"
            with urllib.request.urlopen(url, timeout=30) as response:
                upstream = response.read()
            assert path.read_bytes() == upstream, f"Source differs: {repo}/{relative}"
            return f"{repo}/{relative}", hashlib.sha256(upstream).hexdigest()

        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            hashes = dict(sorted(pool.map(verify, sorted(files))))
        macros = subprocess.check_output([*command, "-E", "-dM", str(source)], text=True)
    definitions = {}
    for line in macros.splitlines():
        _, name, *value = line.split(maxsplit=2)
        definitions[name] = value[0] if value else ""
    expected = {"USE_D_MIN": "", "USE_SIMPLIFIED_TUNING": "", "PID_PROFILE_COUNT": "4",
                "BOARD_NAME": BOARD, "FC_TARGET_MCU": "STM32F405"}
    for name, value in expected.items():
        assert definitions.get(name) == value, name
    assert "USE_TARGET_CONFIG" not in definitions
    result = {"firmwareCommit": commits["betaflight"], "configurationCommit": CONFIG_COMMIT,
              "board": BOARD, "defines": DEFINES, "firmwareIncludes": INCLUDES,
              "effectiveMacros": expected, "undefinedMacros": ["USE_TARGET_CONFIG"],
              "sourceSha256": hashes, "certifiedForRecovery": False}
    rendered = json.dumps(result, indent=2) + "\n"
    output = ROOT / "fixtures/pid-hardware-evidence.json"
    if args.check:
        assert output.read_text() == rendered, "Hardware evidence differs"
    else:
        output.write_text(rendered)
    print(f"{BOARD}: {len(hashes)} pinned inputs verified; D-min/tuning enabled, four profiles, no target reset override.")
    print("Host preprocessing only; no embedded execution or historical backup certification.")


if __name__ == "__main__":
    main()
