"""Certify exact vendor rate reset tables from immutable public source archives.

Developer-only: downloads public firmware, never reads controller backups.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile
import urllib.request
from schema_lookups import lookup_arrays

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'crates/flightlens-core/compatibility/vendor-rate-defaults.json'
RELEASES = [
    ('4.5.3.KAACK_V19', '8cd44381217948c0b2b5087f12e17dde15d6a25c',
     '4.5.0', 'c155f5830d0ffdee1c34071dd21f174ffc374c81'),
    ('2025.12.3-alpha.KAACK_V19', '3b419ca431ba5ea791d7924c8c5b266b911da5b1',
     '2025.12.1', '85d201376a1fc33b223c27448808c2cc7b8f2743'),
]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def archive(repo, commit):
    url = f'https://codeload.github.com/{repo}/tar.gz/{commit}'
    with urllib.request.urlopen(url, timeout=120) as response:
        data = response.read()
    files = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as tar:
        for member in tar:
            path = member.name.partition('/')[2]
            if member.isfile() and (path.startswith(('src/main/', 'mk/')) or path == 'Makefile'):
                files[path] = tar.extractfile(member).read()
    return files, {'url': url, 'sha256': digest(data)}


def function(text, name):
    start = re.search(r'\b(?:void|bool)\s+' + name + r'\s*\([^;]*?\)\s*\{', text, re.S)
    assert start, name
    # The pinned functions have no brace characters in strings/comments.
    depth = 1
    end = start.end()
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return text[start.start():end]


def verify(item):
    version, commit, pack_version, upstream_commit = item
    vendor, vendor_archive = archive('limonspb/betaflight', commit)
    base, base_archive = archive('betaflight/betaflight', upstream_commit)
    pack = json.loads((ROOT / f'crates/flightlens-core/compatibility/betaflight-{pack_version}.json').read_text())
    evidence = {}
    version_header = vendor['src/main/build/version.h'].decode()
    numeric = dict(re.findall(r'^#define\s+(FC_VERSION_\w+)\s+(\d+)\b', version_header, re.M))
    if pack_version == '4.5.0':
        reported = '.'.join(numeric[name] for name in ['FC_VERSION_MAJOR', 'FC_VERSION_MINOR', 'FC_VERSION_PATCH_LEVEL'])
    else:
        reported = '.'.join(numeric[name] for name in ['FC_VERSION_YEAR', 'FC_VERSION_MONTH', 'FC_VERSION_PATCH_LEVEL'])
        assert re.search(r'^#define\s+FC_VERSION_SUFFIX\s+"alpha"', version_header, re.M)
        reported += '-alpha'
    assert '.KAACK_V19"' in version_header and reported + '.KAACK_V19' == version
    evidence['build/version.h'] = {'vendorSha256': digest(vendor['src/main/build/version.h'])}

    def pair(path):
        key = 'src/main/' + path
        evidence[path] = {'vendorSha256': digest(vendor[key]), 'upstreamSha256': digest(base[key])}
        if path in pack['sources']:
            assert digest(base[key]) == pack['sources'][path]['sha256'], (version, path, 'pack anchor')
        return base[key].decode(), vendor[key].decode()

    # Struct layout, enums, axis indexing and the reset macro must be identical.
    for path in ['fc/controlrate_profile.h', 'common/axis.h', 'config/config_reset.h', 'pg/pg.c', 'pg/pg.h']:
        a, b = pair(path)
        assert a == b, (version, path)
    for path, name in [('fc/controlrate_profile.c', 'pgResetFn_controlRateProfiles'),
                       ('config/config.c', 'resetConfig'), ('cli/cli.c', 'cliDefaults')]:
        a, b = pair(path)
        assert function(a, name) == function(b, name), (version, name)
        if name == 'pgResetFn_controlRateProfiles':
            # Existing pack hashes exclude the final newline and closing brace.
            assert digest(function(b, name).rsplit('\n}', 1)[0].encode()) == pack['defaults']['reset_sha256']
    a, b = pair('cli/settings.c')
    mappings = lambda text: [line.strip() for line in text.splitlines()
                             if 'PG_CONTROL_RATE_PROFILES, offsetof' in line]
    assert mappings(a) == mappings(b) and mappings(a), version
    # All lookup arrays used by rate settings, including OFF/ON and rate types.
    tables = set(re.findall(r'config.lookup\s*=\s*\{\s*(\w+)', '\n'.join(mappings(a))))
    ah, bh = pair('cli/settings.h')
    enum_tables = lambda text: re.findall(r'^\s*(TABLE_\w+)(?:\s*=\s*0)?\s*,', text, re.M)
    arrays = []
    for settings, header in [(a, ah), (b, bh)]:
        entries = re.findall(r'^\s*LOOKUP_TABLE_ENTRY\((\w+)\)', settings, re.M)
        names = enum_tables(header)
        assert len(names) == len(entries)
        lookups = lookup_arrays(settings, sized=True)
        arrays.append({name: lookups[entry] for name, entry in zip(names, entries) if name in tables})
    assert arrays[0] == arrays[1] and len(arrays[0]) == len(tables)
    a, b = pair('fc/parameter_names.h')
    names = set(re.findall(r'PARAM_NAME_\w+', '\n'.join(mappings(vendor['src/main/cli/settings.c'].decode()))))
    definitions = lambda text: dict(re.findall(r'^#define\s+(\w+)\s+"([^"\n]+)"', text, re.M))
    assert {name: definitions(a)[name] for name in names} == {name: definitions(b)[name] for name in names}
    # The only non-enum reset constant and profile-count alternatives are equal.
    for path, macro in [('fc/rc_controls.h', 'CONTROL_RATE_CONFIG_RATE_LIMIT_MAX'),
                        ('target/common_pre.h', 'CONTROL_RATE_PROFILE_COUNT')]:
        a, b = pair(path)
        definitions = lambda text: re.findall(r'^\s*#define\s+' + macro + r'\s+([^\n]+)', text, re.M)
        assert definitions(a) == definitions(b) and definitions(a), (version, macro)
        if macro == 'CONTROL_RATE_CONFIG_RATE_LIMIT_MAX':
            assert definitions(b)[0].strip() == pack['defaults']['values']['roll_rate_limit']
    callbacks = []
    constant_locations = []
    for path, raw in vendor.items():
        text = raw.decode(errors='replace')
        if re.search(r'\bvoid\s+targetConfiguration\s*\([^;]*?\)\s*\{', text, re.S):
            callbacks.append(path)
        if re.search(r'^\s*#\s*define\s+CONTROL_RATE_CONFIG_RATE_LIMIT_MAX\b', text, re.M):
            constant_locations.append(path)
    assert not callbacks, (version, callbacks)
    assert constant_locations == ['src/main/fc/rc_controls.h'], constant_locations
    manifest = {path: digest(raw) for path, raw in sorted(vendor.items())}
    return {'version': version, 'commit': commit, 'packVersion': pack_version,
            'upstreamCommit': upstream_commit, 'vendorArchive': vendor_archive,
            'upstreamArchive': base_archive, 'sources': evidence,
            'buildSourceManifestSha256': digest(json.dumps(manifest, sort_keys=True).encode()),
            'targetConfigurationDefinitions': callbacks,
            'resetSha256': pack['defaults']['reset_sha256'], 'values': pack['defaults']['values']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    with ThreadPoolExecutor(max_workers=2) as pool:
        result = list(pool.map(verify, RELEASES))
    output = json.dumps(result, indent=2) + '\n'
    if args.check:
        assert OUT.read_text() == output, 'Vendor evidence changed; regenerate and review'
    else:
        OUT.write_text(output)
    print(f'Verified {len(result)} exact vendor rate-default tables')


if __name__ == '__main__':
    main()
