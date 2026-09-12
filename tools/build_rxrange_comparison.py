"""Developer-only source verification; the application uses the bundled manifest offline."""
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def function(source, name):
    start = source.index('static void ' + name + '(')
    opening = source.index('{', start)
    depth = 1
    end = opening + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return ' '.join(source[start:end].split())


EXPECTED_FUNCTION_HASH = "304027905dcb8c410b213bd74b92a026d6bfa6864beb60c327dab9dd242d32f1"

releases = {**{f'4.5.{n}': 'betaflight-4.5.0-schema-1' for n in range(6)},
            **{f'2025.12.{n}': 'betaflight-2025.12.1-schema-1' for n in range(1, 6)}}
sources = []
for version in releases:
    for path in ['cli/cli.c', 'rx/rx.h']:
        url = f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}'
        data = urllib.request.urlopen(url).read()
        source = data.decode()
        if path == 'cli/cli.c':
            digest = hashlib.sha256(function(source, 'cliRxRange').encode()).hexdigest()
            assert digest == EXPECTED_FUNCTION_HASH, f'Review required: {version} cliRxRange'
        else:
            lines = [' '.join(line.split()) for line in source.splitlines()]
            for prefix in ['#define PWM_PULSE_MIN 750', '#define PWM_PULSE_MAX 2250', '#define NON_AUX_CHANNEL_COUNT 4']:
                assert any(line == prefix or line.startswith(prefix + ' ') for line in lines), (version, prefix)
        sources.append({'version': version, 'url': url, 'sha256': hashlib.sha256(data).hexdigest()})
manifest = {'family': 'betaflight', 'releases': releases, 'channels': 4,
            'min': 750, 'max': 2250, 'sources': sources}
(ROOT / 'src/rxrangeCompatibility.json').write_text(json.dumps(manifest, indent=2) + '\n')
