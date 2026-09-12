"""Developer-only upstream adjustment-range comparison verification; never reads backups."""
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

CLI = ['static void cliAdjustmentRange(', 'static void printAdjustmentRange(', 'static const char *processChannelRangeArgs(']
PATHS = ['cli/cli.c', 'fc/rc_adjustments.c', 'fc/rc_adjustments.h', 'fc/rc_modes.h', 'rx/rx.h']
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
            "static void cliAdjustmentRange(": "228baceebb2e5323abd1ae1d8ae108ed6916b31adbe94bffd18ba21f849beb78",
            "static void printAdjustmentRange(": "f291f7a85648ce3fac2500c22e5e5caedebfa6ac1cc2dee0253a758903813ea5",
            "static const char *processChannelRangeArgs(": "cc8636f6634bbe2de9179b493408e125972e7d5a6248c396327f5d93188c2309"
        },
        "fc/rc_adjustments.c": {
            "file": "52413a3c9a4d016043010df36d76c6bdd01b85310c3aa38b34b9ff105dee934e"
        },
        "fc/rc_adjustments.h": {
            "file": "1a81c0532a3320721f339d7d474944b47a27ec521d2d5eba216360de48195a83"
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
            "static void cliAdjustmentRange(": "01c206e31fa842fdd26ea8e8fb843a3bb7cf90be149406f0719595aa5f8683c5",
            "static void printAdjustmentRange(": "f291f7a85648ce3fac2500c22e5e5caedebfa6ac1cc2dee0253a758903813ea5",
            "static const char *processChannelRangeArgs(": "cc8636f6634bbe2de9179b493408e125972e7d5a6248c396327f5d93188c2309"
        },
        "fc/rc_adjustments.c": {
            "file": "0b37fd80f3a56d4291c9d5c3f97e23605aad74e57bf47263601824f0300dfefe"
        },
        "fc/rc_adjustments.h": {
            "file": "a3feabb885f86d61a76ea58b929abdee18df9dc032826f10076504301cb1b412"
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
(Path(__file__).resolve().parents[1] / 'docs/adjrange-source-evidence.json').write_text(json.dumps(evidence, indent=2) + '\n')
print(f'Verified reviewed baselines across {len(VERSIONS)} releases and {len(evidence)} sources.')
