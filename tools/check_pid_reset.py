"""Execute the complete pinned 4.5.0 PID profile reset with upstream SITL headers.

Usage: python3 tools/check_pid_reset.py /path/to/betaflight/source
Requires CC (default cc). No backups are read. Executes reset with/without D-min
and reset plus default simplified tuning. This does not execute embedded code or
custom target callbacks. The dispatcher runs with a PID-only host registry.
"""
import argparse
import hashlib
import json
import os
import shlex
from pathlib import Path
import subprocess
import tempfile

from check_pid_tuning import one
from verify_pid_defaults import ROOT, function


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    args = parser.parse_args()
    evidence = json.loads((ROOT / "fixtures/pid-default-evidence.json").read_text())[0]
    sources = {}
    verified = set()
    for path, expected in evidence["sourceSha256"].items():
        relative = path if path == "Makefile" or path.startswith("mk/") else f"src/main/{path}"
        local = (args.source / relative).resolve()
        raw = local.read_bytes()
        assert hashlib.sha256(raw).hexdigest() == expected, path
        verified.add(local)
        sources[path] = raw.decode()
    tuning_header = sources["config/simplified_tuning.h"]
    enum = one(r"typedef enum \{\s*PID_SIMPLIFIED_TUNING_OFF = 0,.*?\} [^;]+;", tuning_header)
    code = "\n".join([
        '#include <stdint.h>\n#include <string.h>\n#include <assert.h>\n#include <stdio.h>',
        '#include "fc/core.h"\n#include "sensors/battery.h"\n#include "flight/pid.h"',
        '#include "config/config_reset.h"\n#include "pg/pg_ids.h"',
        '#if !defined(USE_D_MIN) || defined(USE_TARGET_CONFIG)\n#error Unexpected SITL feature selection\n#endif',
        *[f"#define {name} {evidence['macros'][name]}" for name in
          ["SIMPLIFIED_TUNING_DEFAULT", "SIMPLIFIED_TUNING_D_DEFAULT"]],
        enum,
        '#ifdef FLIGHTLENS_WITHOUT_D_MIN\n#undef USE_D_MIN\n#endif',
        one(r"^static inline int constrain\([^\n]+\)\n\{.*?^\}", sources["common/maths.h"]),
        one(r"^PG_REGISTER_ARRAY_WITH_RESET_FN\(pidProfile_t, PID_PROFILE_COUNT, pidProfiles, PG_PID_PROFILE, 8\);", sources["flight/pid.c"]),
        function(sources["flight/pid.c"], "resetPidProfile"),
        function(sources["flight/pid.c"], "pgResetFn_pidProfiles"),
        *[function(sources["pg/pg.c"], name) for name in
          ["pgOffset", "pgResetInstance", "pgReset", "pgResetAll"]],
        '#ifdef FLIGHTLENS_APPLY_TUNING',
        function(sources["config/simplified_tuning.c"], "calculateNewPidValues"),
        function(sources["config/simplified_tuning.c"], "applySimplifiedTuningPids"),
        '#endif',
        '''int main(void) {
            pidProfile_t *profiles = pidProfiles_SystemArray;
            assert(PG_REGISTRY_SIZE == 1);
            assert(pgN(&pidProfiles_Registry) == PG_PID_PROFILE);
            assert(pgSize(&pidProfiles_Registry) == sizeof(pidProfiles_SystemArray));
            #ifdef USE_D_MIN
            const unsigned expected[3][4] = {{45,80,40,120},{47,84,46,125},{45,80,0,120}};
            #else
            const unsigned expected[3][4] = {{45,80,30,120},{47,84,32,125},{45,80,0,120}};
            #endif
            assert(PID_PROFILE_COUNT == 4);
            for (unsigned fill=0; fill<2; ++fill) {
                memset(profiles, fill ? 0xff : 0, sizeof(pidProfiles_SystemArray));
                pgResetAll();
                for (unsigned p=0; p<PID_PROFILE_COUNT; ++p) {
                    #ifdef FLIGHTLENS_APPLY_TUNING
                    applySimplifiedTuningPids(&profiles[p]);
                    #endif
                    for (unsigned axis=0; axis<3; ++axis) {
                        const pidf_t result = profiles[p].pid[axis];
                        assert(result.P == expected[axis][0] && result.I == expected[axis][1]);
                        assert(result.D == expected[axis][2] && result.F == expected[axis][3]);
                    }
                    assert(profiles[p].simplified_pids_mode == PID_SIMPLIFIED_TUNING_RPY);
                    assert(profiles[p].simplified_master_multiplier == SIMPLIFIED_TUNING_DEFAULT);
                }
            }
            puts("PID-only registry dispatch, four SITL profiles: reset gains match from zero and nonzero memory.");
            return 0;
        }''',
    ])
    with tempfile.TemporaryDirectory(prefix="flightlens-reset-") as temp:
        source = Path(temp) / "reset.c"
        binary = Path(temp) / "reset"
        source.write_text(code)
        linker = Path(temp) / "registry.ld"
        linker.write_text("""SECTIONS {
            .pg_registry : {
                __pg_registry_start = .;
                KEEP(*(.pg_registry))
                __pg_registry_end = .;
            }
            .pg_resetdata : {
                __pg_resetdata_start = .;
                __pg_resetdata_end = .;
            }
        } INSERT AFTER .data;
        """)
        compiler = os.environ.get("CC", "cc")
        includes = ["-I", str(args.source / "src/main"), "-I", str(args.source / "src/main/target/SITL")]
        # Compiler-discovered upstream dependencies must all be pinned above.
        dependencies = subprocess.check_output([compiler, "-MM", "-DSIMULATOR_BUILD", *includes, str(source)], text=True)
        for dependency in shlex.split(dependencies.replace("\\\n", "").split(":", 1)[1]):
            local = Path(dependency).resolve()
            assert local == source or local in verified, f"Unpinned header: {local}"
        for variant in [[], ["-DFLIGHTLENS_WITHOUT_D_MIN"], ["-DFLIGHTLENS_APPLY_TUNING"]]:
            for optimization in ["-O0", "-O2", "-Ofast"]:
                subprocess.run([compiler, "-std=c11", optimization, "-DSIMULATOR_BUILD", *variant,
                                "-Wall", "-Wextra", "-Werror", "-fsanitize=undefined,float-cast-overflow",
                                "-fno-sanitize-recover=all", *includes, str(source), f"-Wl,-T,{linker}", "-o", str(binary)], check=True)
                result = subprocess.check_output([str(binary)], text=True).strip()
                print(f"{variant or ['reset with D-min']} {optimization}: {result}")
    print("Ten gains agree across reset feature variants and default tuning; roll/pitch D differ. Host execution with real SITL types, not embedded execution.")


if __name__ == "__main__":
    main()
