"""Generate offline fixtures from pinned, unmodified Betaflight C blocks.

Developer-only network use: fetch public firmware source, never backup data.
Run: python3 tools/build_throttle_vectors.py (requires cc).
"""
import hashlib
import json
import pathlib
import re
import subprocess
import tempfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]


def block(source, marker):
    start = source.index(marker)
    opening = source.index('{', start)
    depth = 1
    end = opening + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]


# Keep the immutable release identities in the reviewed evidence document.
releases = re.findall(r'\| (4\.[2345]\.\d+) \| `([a-f0-9]{40})`',
                      (ROOT / 'docs/throttle-curve-preview.md').read_text())
assert len(releases) == 9
result = []
for version, commit in releases:
    sources = {}
    hashes = {}
    for path in ['fc/rc.c', 'flight/mixer.c']:
        url = f'https://raw.githubusercontent.com/betaflight/betaflight/{commit}/src/main/{path}'
        data = urllib.request.urlopen(url, timeout=30).read()
        sources[path] = data.decode()
        hashes[path] = hashlib.sha256(data).hexdigest()
    rc = sources['fc/rc.c']
    loop = block(rc[rc.index('void initRcProcessing(void)'):],
                 'for (int i = 0; i < THROTTLE_LOOKUP_LENGTH; i++)')
    lookup = block(rc, 'static int16_t rcLookupThrottle(')
    limit = block(sources['flight/mixer.c'], 'static float applyThrottleLimit(')
    shim = '''#include <stdint.h>
#include <stdio.h>
#define THROTTLE_LOOKUP_LENGTH 12
#define PWM_RANGE_MIN 1000
#define PWM_RANGE_MAX 2000
#define PWM_RANGE 1000
#define RPM_LIMIT_ACTIVE 0
#define THROTTLE_LIMIT_TYPE_SCALE 1
#define THROTTLE_LIMIT_TYPE_CLIP 2
#define MIN(a,b) ((a)<(b)?(a):(b))
static int16_t lookupThrottleRC[THROTTLE_LOOKUP_LENGTH];
struct Profile { uint8_t thrMid8, thrExpo8, throttle_limit_type, throttle_limit_percent; };
static struct Profile profile;
static struct Profile *currentControlRateProfile = &profile;
'''
    main = '''
int main(void) {
int cases[][2] = {{50,0},{50,50},{50,100},{30,70},{0,100},{1,100},{99,100},{73,37}};
int percentages[] = {25,50,73,100};
for(int c=0;c<8;c++) {
profile.thrMid8=cases[c][0]; profile.thrExpo8=cases[c][1];
initialize();
for(int mode=0;mode<3;mode++) for(int p=0;p<4;p++) {
profile.throttle_limit_type=mode; profile.throttle_limit_percent=percentages[p];
for(int t=0;t<=1000;t++) {
if(t%100>1 && t%100<99 && t!=250 && t!=750) continue;
int command=rcLookupThrottle(t)-PWM_RANGE_MIN;
printf("%d,%d,%d,%d,%d,%d,%.9g\\n",cases[c][0],cases[c][1],mode,percentages[p],t,command,
applyThrottleLimit(command/1000.0f)*100.0f);
}
}}
}
'''
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp)
        (path / 'reference.c').write_text(shim + '\nvoid initialize(void) {\n' + loop + '\n}\n' + lookup + '\n' + limit + main)
        subprocess.run(['cc', '-O0', '-fsanitize=undefined', '-fno-sanitize-recover=all',
                        str(path / 'reference.c'), '-o', str(path / 'reference')], check=True)
        output = subprocess.check_output([str(path / 'reference')], text=True)
    vectors = []
    for line in output.splitlines():
        mid, expo, mode, percent, t, command, y = line.split(',')
        vectors.append([int(mid), int(expo), int(mode), int(percent), int(t), int(command), float(y)])
    result.append({'version': version, 'commit': commit, 'sourceSha256': hashes,
                   'columns': ['mid', 'expo', 'mode', 'percent', 'input', 'command', 'outputPercent'],
                   'vectors': vectors})
    print(f'{version}: {len(vectors)} upstream C vectors')
(ROOT / 'fixtures/throttle-vectors.json').write_text(json.dumps(result, separators=(',', ':')) + '\n')
