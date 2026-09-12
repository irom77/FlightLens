"""Developer-only upstream VTX comparison verification; never reads backups."""
import concurrent.futures, hashlib, json, urllib.request
from pathlib import Path

def block(source, start):
    start = source.index(start)
    opening = source.index('{', start)
    depth, end = 1, opening + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return ' '.join(source[start:end].split())

CLI = ['static void cliVtx(', 'static void printVtx(', 'static void cliVtxTable(', 'static void printVtxTable(', 'static const char *printVtxTableBand(', 'static const char *printVtxTablePowerValues(', 'static const char *printVtxTablePowerLabels(', 'static char *formatVtxTableBandFrequency(', 'static const char *processChannelRangeArgs(']
PATHS = ['cli/cli.c', 'drivers/vtx_table.c', 'drivers/vtx_table.h', 'pg/vtx_table.c', 'pg/vtx_table.h', 'io/vtx_control.c', 'io/vtx_control.h', 'fc/rc_modes.h', 'rx/rx.h']
VERSIONS = [f'4.5.{n}' for n in range(6)] + [f'2025.12.{n}' for n in range(1, 6)]
def fetch(item):
    version, path = item
    url = f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}'
    data = urllib.request.urlopen(url, timeout=30).read()
    source = data.decode()
    sections = {prefix: block(source, prefix) for prefix in CLI} if path == 'cli/cli.c' else {'file': ' '.join(source.split())}
    return dict(version=version, path=path, url=url, sha256=hashlib.sha256(data).hexdigest(), sections={k: hashlib.sha256(v.encode()).hexdigest() for k,v in sections.items()})
# Reviewed release-family baselines; update only after inspecting upstream differences.
EXPECTED = {
    "4.5.0": {
        "cli/cli.c": {
            "static void cliVtx(": "dba659244e7ec0c9d69505a2a2b6e0870727c050ae58b6c56209dfbd8f7330a6",
            "static void printVtx(": "c1dc102a6248aeaf1b25bfd991c77e2207730b63ab00fa8602c18f90b6a3d7e5",
            "static void cliVtxTable(": "8c847747a39eab708f39c327c972df96c9ad6c376af115b790202acb7162d48a",
            "static void printVtxTable(": "07ced62d45f071c84c8fd251df9c792a30113c5eeaf10d1a70b468a6e845424f",
            "static const char *printVtxTableBand(": "0edcb451ef45fcb7087857a3736b45addfd8bd55556bf0c4b0c3448d4341b7b8",
            "static const char *printVtxTablePowerValues(": "2fe2661404995d7002ff45c79aaf41150a3c9e7244a000337fa7bd0b12b15772",
            "static const char *printVtxTablePowerLabels(": "590a06f9344ac779cdfb278037a3dd768892705ba9eae0dd375fb46bd1ead859",
            "static char *formatVtxTableBandFrequency(": "5c55c9a4345745a51c3637993885fb25c0c900e90367791c18294e6a2a9c2d13",
            "static const char *processChannelRangeArgs(": "cc8636f6634bbe2de9179b493408e125972e7d5a6248c396327f5d93188c2309"
        },
        "drivers/vtx_table.c": {
            "file": "82f3f125ecb424e5a9023e3f8dd5b4ad2151ca40e92a661e8e8f866efd5df549"
        },
        "drivers/vtx_table.h": {
            "file": "1432f415e8bfe3ff9b6a3b54efb85d8537a3f23b96e04e7ed2f781ff8c2cfce8"
        },
        "pg/vtx_table.c": {
            "file": "50d42bc895cc3853e51e331a4db4e3697902382dd2943d9029199c86066084dc"
        },
        "pg/vtx_table.h": {
            "file": "5cb6c068ca8b0b14b88eac85a383835dbcfc39092442b518b813160accf4e0f7"
        },
        "io/vtx_control.c": {
            "file": "809a0ce4dc183f2a8da998b00538903ca40794e0aaaaea3cd1eaeed4eda009f4"
        },
        "io/vtx_control.h": {
            "file": "a25ab9187a26e7832e5835f034af62f4217da71432fa5c9ef3ee9caeeca45736"
        },
        "fc/rc_modes.h": {
            "file": "b896e9a07bdb8adc61bc8c149af4dd084b8227a5b183deb7982cc22f9f635e10"
        },
        "rx/rx.h": {
            "file": "8b51d8963a698fde4ff9d3df327e3aff90045657331ce4f77c628583947d7ef7"
        }
    },
    "2025.12.1": {
        "cli/cli.c": {
            "static void cliVtx(": "463bd917b094f427a0270f5d143751e6c191046f60aae4aec3175f549c14dccf",
            "static void printVtx(": "c1dc102a6248aeaf1b25bfd991c77e2207730b63ab00fa8602c18f90b6a3d7e5",
            "static void cliVtxTable(": "8c847747a39eab708f39c327c972df96c9ad6c376af115b790202acb7162d48a",
            "static void printVtxTable(": "07ced62d45f071c84c8fd251df9c792a30113c5eeaf10d1a70b468a6e845424f",
            "static const char *printVtxTableBand(": "0edcb451ef45fcb7087857a3736b45addfd8bd55556bf0c4b0c3448d4341b7b8",
            "static const char *printVtxTablePowerValues(": "2fe2661404995d7002ff45c79aaf41150a3c9e7244a000337fa7bd0b12b15772",
            "static const char *printVtxTablePowerLabels(": "590a06f9344ac779cdfb278037a3dd768892705ba9eae0dd375fb46bd1ead859",
            "static char *formatVtxTableBandFrequency(": "5c55c9a4345745a51c3637993885fb25c0c900e90367791c18294e6a2a9c2d13",
            "static const char *processChannelRangeArgs(": "cc8636f6634bbe2de9179b493408e125972e7d5a6248c396327f5d93188c2309"
        },
        "drivers/vtx_table.c": {
            "file": "edee37f130764cab815ef8806e3e2f0bc79a93b8fd53ee8a9698de8bf98d0c54"
        },
        "drivers/vtx_table.h": {
            "file": "8dfa5b6829e9b9d5470bd581c11b1536e5d3af4e49c3970152769ba0d7c76c6d"
        },
        "pg/vtx_table.c": {
            "file": "50d42bc895cc3853e51e331a4db4e3697902382dd2943d9029199c86066084dc"
        },
        "pg/vtx_table.h": {
            "file": "5cb6c068ca8b0b14b88eac85a383835dbcfc39092442b518b813160accf4e0f7"
        },
        "io/vtx_control.c": {
            "file": "809a0ce4dc183f2a8da998b00538903ca40794e0aaaaea3cd1eaeed4eda009f4"
        },
        "io/vtx_control.h": {
            "file": "c416b02b14627673e0c7072cb6942a420906757a9be60bcb32353dc27d2973c1"
        },
        "fc/rc_modes.h": {
            "file": "a75e96c6a1c924ecad9736db01164ba5861ac3ee297639ece75896e21d155261"
        },
        "rx/rx.h": {
            "file": "cdb077d3a889a7e5528f0012c3565aecc5bbbf977153e8817df2385dd7a58d11"
        }
    }
}

with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
    evidence = list(pool.map(fetch, [(v,p) for v in VERSIONS for p in PATHS]))
for row in evidence:
    baseline = '4.5.0' if row['version'].startswith('4.5.') else '2025.12.1'
    if row['sections'] != EXPECTED[baseline][row['path']]:
        raise RuntimeError(f"Review required: {row['version']} {row['path']}")
(Path(__file__).resolve().parents[1] / 'docs/vtx-source-evidence.json').write_text(json.dumps(evidence, indent=2) + '\n')
print(f'Verified reviewed baselines across {len(VERSIONS)} releases and {len(evidence)} sources.')
