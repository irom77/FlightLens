"""Developer-only verification of reviewed rxfail source; never reads backups."""
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

def sections(path, source):
    if path == 'cli/cli.c':
        return {name: block(source, prefix + name) for name, prefix in [
            ('cliRxFailsafe', 'static void '),
            ('printRxFailsafe', 'static void '),
            ('rxFailsafeModesTable', 'static const rxFailsafeChannelMode_e '),
        ]} | {'modeCharacters': next(' '.join(line.split()) for line in source.splitlines() if line.startswith('static const char rxFailsafeModeCharacters'))}
    if path == 'rx/rx.c':
        return {'getRxfailValue': block(source, 'static uint16_t getRxfailValue')}
    names = ['PWM_PULSE_MIN', 'PWM_PULSE_MAX', 'RXFAIL_STEP_TO_CHANNEL_VALUE', 'CHANNEL_VALUE_TO_RXFAIL_STEP', 'MAX_RXFAIL_RANGE_STEP', 'MAX_SUPPORTED_RC_CHANNEL_COUNT', 'NON_AUX_CHANNEL_COUNT']
    return {name: next(' '.join(line.split('//')[0].split()) for line in source.splitlines() if line.startswith('#define ' + name + ' ') or line.startswith('#define ' + name + '(')) for name in names} | {
        'modeEnum': block(source, 'typedef enum {\n    RX_FAILSAFE_MODE_AUTO'),
        'typeEnum': block(source, 'typedef enum {\n    RX_FAILSAFE_TYPE_FLIGHT')
    }

def fetch(item):
    version, path = item
    url = f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}'
    data = urllib.request.urlopen(url, timeout=30).read()
    checks = {name: hashlib.sha256(value.encode()).hexdigest() for name, value in sections(path, data.decode()).items()}
    return {'version': version, 'path': path, 'url': url, 'sha256': hashlib.sha256(data).hexdigest(), 'sections': checks}

EXPECTED = {
    "cli/cli.c": {
        "cliRxFailsafe": "905398eb531e633371665754a5ca531a333ee553c7a53c45508293772abe06cd",
        "printRxFailsafe": "b5de46b049d7768493fa9f692cb3fb37ba1267d868d9f5f2603db6b222cfd09c",
        "rxFailsafeModesTable": "8b8052a91855cc17f58bc9e7398710fb058ae75dd8e9f7c546aa5ee4a065ed3c",
        "modeCharacters": "617f9cd4febf792dd3f97cea04f3b592dfb6442e5950007e8c6fd3142a1afdb5"
    },
    "rx/rx.h": {
        "PWM_PULSE_MIN": "1d05f3104c6d8397ecec333da6ce333b48862df8a9fee48a7797e0a0a959beda",
        "PWM_PULSE_MAX": "4a58bfdd394d2b408e2875dfad12b12a000dc2fdcd95ad5b22e1cd0d134c28f7",
        "RXFAIL_STEP_TO_CHANNEL_VALUE": "56d71823500d0cec6497ad76494f3ab1f6abd7c7dacd238e77cfa6c37d3cccb3",
        "CHANNEL_VALUE_TO_RXFAIL_STEP": "5851ea599a23ab412e82abca6a968a14fbfc91ce4b4aa86838322b52c8de537c",
        "MAX_RXFAIL_RANGE_STEP": "a50b46570e1bdb88e87ff65fffad905c35f9fc0a41238ef343648b83b5026c4b",
        "MAX_SUPPORTED_RC_CHANNEL_COUNT": "bddf96f7997f1547fbcf3750c5d4723732b9865773b88aeb5c8395dd93137c54",
        "NON_AUX_CHANNEL_COUNT": "aaee172bfb7c7b4cb9063d5401f970f1ec5ece09649dfadf290e4451fc85bc5b",
        "modeEnum": "3a3939f83475debd2a90879e4fb2e162d5bd222eaa624db78e570de2e18e967c",
        "typeEnum": "caf7a7cb9030f651b8ff9aeb0a21f067e06c7369be491958088fd6f28e37ce62"
    },
    "rx/rx.c": {
        "getRxfailValue": "3356648c017e017c65e06a51b596cfe5514dc40244ac3fba29368b934c3f848f"
    }
}

versions = [f'4.5.{n}' for n in range(6)] + [f'2025.12.{n}' for n in range(1,6)]
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
    sources = list(pool.map(fetch, [(v,p) for v in versions for p in ['cli/cli.c','rx/rx.h','rx/rx.c']]))
for source in sources:
    assert source['sections'] == EXPECTED[source['path']], f"Review required: {source['version']} {source['path']}"
(Path(__file__).resolve().parents[1] / 'docs/rxfail-source-evidence.json').write_text(json.dumps(sources, indent=2)+'\n')
print(f'Verified identical reviewed sections across {len(versions)} releases and {len(sources)} sources.')
