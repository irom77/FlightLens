"""Verify the source boundary for the ten build-invariant 4.5 PID gains.

Downloads immutable public release archives only. No backups are read.
The contract is unmodified upstream source/build machinery, as advertised by a
plain release identity; it cannot authenticate a custom binary using that name.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import json
import re
import tarfile
import urllib.request

from verify_pid_defaults import ROOT, RELEASES


def verify(item):
    version, commit = item
    url = f"https://codeload.github.com/betaflight/betaflight/tar.gz/{commit}"
    with urllib.request.urlopen(url, timeout=120) as response:
        archive = response.read()
    evidence = next(r for r in json.loads((ROOT / "fixtures/pid-default-evidence.json").read_text()) if r["version"] == version)
    assert evidence["commit"] == commit and not evidence["changedFunctionsFrom450"]
    manifest = {}
    macro_locations = {}
    callbacks = []
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
        for member in tar:
            path = member.name.partition("/")[2]
            if not member.isfile() or not (path.startswith("src/main/") or path.startswith("mk/") or path == "Makefile"):
                continue
            raw = tar.extractfile(member).read()
            manifest[path] = hashlib.sha256(raw).hexdigest()
            text = raw.decode("utf8", errors="replace")
            # A board callback needs added source: none is implemented by these
            # releases. mk/config.mk selects only config.h, not external C files.
            if re.search(r"\bvoid\s+targetConfiguration\s*\([^;]*?\)\s*\{", text, re.S):
                callbacks.append(path)
            for name in re.findall(r"^\s*#\s*define\s+((?:PID_(?:ROLL|PITCH|YAW)|SIMPLIFIED_TUNING(?:_D)?)_DEFAULT)\b", text, re.M):
                macro_locations.setdefault(name, []).append(path)
    for path, sha in evidence["sourceSha256"].items():
        relative = path if path == "Makefile" or path.startswith("mk/") else f"src/main/{path}"
        assert manifest[relative] == sha, (version, relative)
    assert not callbacks, (version, callbacks)
    assert len(macro_locations) == 5
    for name, locations in macro_locations.items():
        expected = "src/main/flight/pid.h" if name.startswith("PID_") else "src/main/config/simplified_tuning.h"
        assert locations == [expected], (version, name, locations)
    return {"version": version, "commit": commit, "archiveUrl": url,
            "buildSourceFiles": len(manifest),
            "buildSourceManifestSha256": hashlib.sha256(json.dumps(manifest, sort_keys=True).encode()).hexdigest(),
            "targetConfigurationDefinitions": callbacks, "defaultMacroLocations": macro_locations}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    with ThreadPoolExecutor(max_workers=2) as pool:
        releases = list(pool.map(verify, RELEASES.items()))
    # Only the ten values unchanged with/without D-min and simplified tuning.
    evidence = json.loads((ROOT / "fixtures/pid-default-evidence.json").read_text())
    baseline = evidence[0]
    values = {}
    for axis in ["roll", "pitch", "yaw"]:
        gains = re.findall(r"\d+", baseline["macros"][f"PID_{axis.upper()}_DEFAULT"])
        assert len(gains) == 4
        for gain, value in zip(["p", "i", "d", "f"], gains):
            if gain != "d" or axis == "yaw":
                values[f"{gain}_{axis}"] = value
    assert len(values) == 10
    for release in evidence:
        assert release["macros"] == baseline["macros"]
        assert release["cliMappings"] == baseline["cliMappings"]
    result = {"sourceVersion": "4.5.0", "verified": list(RELEASES), "values": values,
              "contract": "Unmodified upstream release source and build machinery; ten gains invariant across D-min and simplified-tuning selection. No arbitrary custom source or target callbacks.",
              "releases": releases}
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    output = ROOT / "crates/flightlens-core/compatibility/pid-defaults.json"
    if args.check:
        assert output.read_text() == rendered, "PID build evidence differs"
    else:
        output.write_text(rendered)
    print(f"{len(releases)} release archives verified; ten invariant PID gains recorded.")


if __name__ == "__main__":
    main()
