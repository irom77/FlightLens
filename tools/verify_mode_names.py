"""Generate/check offline mode tables from immutable public firmware source only."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'crates/flightlens-core/compatibility/mode-names.json'
MODERN = {
    '2025.12.1': '85d201376a1fc33b223c27448808c2cc7b8f2743',
    '2025.12.2': '79065c96ba0bb5cdc675e67d7093e05dab8b330e',
    '2025.12.3': 'db7df6e48b9727d5984e18c906bf0e4769b2abf1',
    '2025.12.4': 'c2af58a0cbe060bd44ee2eb53baf0f42baf7d15e',
    '2025.12.5': '7348054f268f0058574719c134e9f149565bb8ea',
}


def fetch(release):
    version, commit, repo = release
    url = f'https://raw.githubusercontent.com/{repo}/{commit}/src/main/msp/msp_box.c'
    data = urllib.request.urlopen(url, timeout=30).read()
    source = data.decode()
    table = source.split('static const box_t boxes[CHECKBOX_ITEM_COUNT] = {', 1)[1].split('\n};', 1)[0]
    # Anchoring excludes historical entries commented out by upstream.
    entries = re.findall(r'^\s*\{\s*(?:\.boxId = )?\w+,\s*(?:\.boxName = )?"([^"]+)",\s*(?:\.permanentId = )?(\d+)', table, re.M)
    names = {id_: name for name, id_ in entries}
    assert len(names) == len(entries) and len(names) >= 40
    return version, {'url': url, 'sha256': hashlib.sha256(data).hexdigest(), 'names': names}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    releases = re.findall(r'\| (4\.[2345]\.\d+) \| `([a-f0-9]{40})`', (ROOT / 'docs/throttle-curve-preview.md').read_text())
    assert len(releases) == 25
    releases = [(v, c, 'betaflight/betaflight') for v, c in releases + list(MODERN.items())]
    releases += [
        ('4.5.3.KAACK_V19', '8cd44381217948c0b2b5087f12e17dde15d6a25c', 'limonspb/betaflight'),
        ('2025.12.3-alpha.KAACK_V19', '3b419ca431ba5ea791d7924c8c5b266b911da5b1', 'limonspb/betaflight'),
    ]
    with ThreadPoolExecutor(max_workers=8) as pool:
        result = dict(pool.map(fetch, releases))
    output = json.dumps(result, indent=2) + '\n'
    if args.check:
        assert OUT.read_text() == output, 'Mode evidence differs; regenerate and review'
    else:
        OUT.write_text(output)
    print(f'Verified {len(result)} release mode tables')


if __name__ == '__main__':
    main()
