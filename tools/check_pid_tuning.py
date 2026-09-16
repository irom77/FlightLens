"""Execute pinned upstream PID tuning with a reduced, source-checked profile.

Usage: python3 tools/check_pid_tuning.py /path/to/betaflight/source
No backups are read. Requires a C compiler (CC, default cc).
This checks arithmetic, not target/build certification or the full reset path.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

from verify_pid_defaults import ROOT, function


def one(pattern, source):
    matches = re.findall(pattern, source, re.M | re.S)
    assert len(matches) == 1, pattern
    return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    args = parser.parse_args()
    evidence = json.loads((ROOT / "fixtures/pid-default-evidence.json").read_text())[0]
    sources = {}
    for path, expected in evidence["sourceSha256"].items():
        relative = path if path == "Makefile" or path.startswith("mk/") else f"src/main/{path}"
        raw = (args.source / relative).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == expected, path
        sources[path] = raw.decode()
    header = sources["flight/pid.h"]
    assert re.search(r"^#define SIMPLIFIED_TUNING_PIDS_MIN 0$", sources["config/simplified_tuning.h"], re.M)
    assert re.search(r"^#define SIMPLIFIED_TUNING_MAX 200$", sources["config/simplified_tuning.h"], re.M)
    fields = re.findall(r"^    uint8_t (simplified_\w+);", header, re.M)
    assert len(fields) == 11, fields
    pidf = one(r"typedef struct pidf_s \{.*?\} pidf_t;", header)
    d_min = one(r"^    uint8_t d_min\[XYZ_AXIS_COUNT\];[^\n]*", header)
    constrain = one(r"^static inline int constrain\([^\n]+\)\n\{.*?^\}", sources["common/maths.h"])
    axes = sources["common/axis.h"]
    assert re.search(r"FD_ROLL = 0,\s*FD_PITCH,\s*FD_YAW", axes)
    assert re.search(r"PID_ROLL,\s*PID_PITCH,\s*PID_YAW", header)
    enum = one(r"typedef enum \{\s*PID_SIMPLIFIED_TUNING_OFF = 0,.*?\} [^;]+;", sources["config/simplified_tuning.h"])
    macros = dict(evidence["macros"])
    for name in ["PID_GAIN_MAX", "F_GAIN_MAX"]:
        macros[name] = one(r"^#define " + name + r" ([^\n]+)", header)
    initializers = []
    for field in fields:
        value = one(r"^        \." + field + r" = ([^,\n]+),", sources["flight/pid.c"])
        initializers.append(f".{field} = {value},")
    code = "\n".join([
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n#include <assert.h>",
        "#define USE_D_MIN\n#define XYZ_AXIS_COUNT 3\n#define FLIGHT_DYNAMICS_INDEX_COUNT 3",
        "enum { FD_ROLL, FD_PITCH, FD_YAW };\nenum { PID_ROLL, PID_PITCH, PID_YAW };",
        *[f"#define {key} {value}" for key, value in macros.items()],
        pidf, enum, "typedef struct { pidf_t pid[3];", d_min,
        *[f"uint8_t {field};" for field in fields], "} pidProfile_t;", constrain,
        function(sources["config/simplified_tuning.c"], "calculateNewPidValues"),
        function(sources["config/simplified_tuning.c"], "applySimplifiedTuningPids"),
        "int main(void) { pidProfile_t profile = {",
        ".pid = { PID_ROLL_DEFAULT, PID_PITCH_DEFAULT, PID_YAW_DEFAULT },",
        ".d_min = D_MIN_DEFAULT,", *initializers, "};",
        "applySimplifiedTuningPids(&profile);",
        'for (int i=0; i<3; ++i) printf("%u %u %u %u\\n", (unsigned)profile.pid[i].P, (unsigned)profile.pid[i].I, (unsigned)profile.pid[i].D, (unsigned)profile.pid[i].F);',
        "const pidProfile_t baseline = profile;",
        "for (int mode=0; mode<3; ++mode) {",
        *["""for (int value=0; value<=200; ++value) {
            profile = baseline;
            profile.simplified_pids_mode = mode;
            profile.FIELD = value;
            applySimplifiedTuningPids(&profile);
            for (int axis=0; axis<3; ++axis) {
                const pidf_t result = profile.pid[axis];
                assert(result.P <= PID_GAIN_MAX && result.I <= PID_GAIN_MAX);
                assert(result.D <= PID_GAIN_MAX && result.F <= F_GAIN_MAX);
                if (mode == PID_SIMPLIFIED_TUNING_OFF || (mode == PID_SIMPLIFIED_TUNING_RP && axis == PID_YAW)) {
                    assert(result.P == baseline.pid[axis].P && result.I == baseline.pid[axis].I);
                    assert(result.D == baseline.pid[axis].D && result.F == baseline.pid[axis].F);
                }
                printf("%u %u %u %u\\n", (unsigned)result.P, (unsigned)result.I, (unsigned)result.D, (unsigned)result.F);
            }
        }""".replace("FIELD", field) for field in fields
          if field not in {"simplified_pids_mode", "simplified_dterm_filter", "simplified_dterm_filter_multiplier"}],
        "}", "return 0; }",
    ])
    with tempfile.TemporaryDirectory(prefix="flightlens-pid-") as temp:
        source = Path(temp) / "pid.c"
        binary = Path(temp) / "pid"
        source.write_text(code)
        outputs = []
        for optimization in ["-O0", "-O2", "-Ofast"]:
            subprocess.run([os.environ.get("CC", "cc"), "-std=c11", optimization,
                            "-Wall", "-Wextra", "-Werror", "-fsanitize=undefined,float-cast-overflow",
                            "-fno-sanitize-recover=all", str(source), "-o", str(binary)], check=True)
            outputs.append(subprocess.check_output([str(binary)], text=True))
        expected = "45 80 40 120\n47 84 46 125\n45 80 0 120\n"
        assert all(output.startswith(expected) for output in outputs)
        assert all(len(output.splitlines()) == 3 + 3 * 8 * 201 * 3 for output in outputs)
        for optimization, output in zip(["O0", "O2", "Ofast"], outputs):
            differences = sum(a != b for a, b in zip(outputs[0].splitlines(), output.splitlines()))
            print(f"{optimization}: 4824 profiles passed range/mode assertions; {differences} axis rows differ from O0")
            if differences:
                tuning_fields = [field for field in fields if field not in {
                    "simplified_pids_mode", "simplified_dterm_filter", "simplified_dterm_filter_multiplier"}]
                for row, (a, b) in enumerate(zip(outputs[0].splitlines(), output.splitlines())):
                    if a != b:
                        profile_index, axis = divmod(row - 3, 3)
                        mode, remainder = divmod(profile_index, 8 * 201)
                        field_index, value = divmod(remainder, 201)
                        print(f"  First difference: mode={mode}, {tuning_fields[field_index]}={value}, axis={axis}: O0 [{a}], {optimization} [{b}]")
                        break
        print("Pinned 4.5.0 default simplified tuning, USE_D_MIN, O0/O2/Ofast agree:")
        print(expected, end="")
        print("Arithmetic evidence only; target/build certification remains pending.")


if __name__ == "__main__":
    main()
