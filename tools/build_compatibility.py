"""Explicit developer-only pack generation from pinned upstream tags; never runs in-app."""
import hashlib, json, pathlib, re, urllib.request
ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / 'crates/flightlens-core/compatibility'
OUT.mkdir(parents=True, exist_ok=True)
CONSTANTS = {'UINT8_MAX':255,'UINT16_MAX':65535,'INT8_MIN':-128,'INT8_MAX':127,
 'VTX_TABLE_MAX_BANDS':8,'VTX_TABLE_MAX_CHANNELS':8,'VTX_TABLE_MAX_POWER_LEVELS':8,'VTX_TABLE_MIN_USER_FREQ':5000,'VTX_TABLE_MAX_USER_FREQ':5999,'PID_GAIN_MAX':250,'CONTROL_RATE_CONFIG_RC_RATES_MAX':255,'CONTROL_RATE_CONFIG_RC_EXPO_MAX':100,
 'CONTROL_RATE_CONFIG_RATE_MAX':255,'CONTROL_RATE_CONFIG_RATE_LIMIT_MIN':200,
 'CONTROL_RATE_CONFIG_RATE_LIMIT_MAX':1998,'OSD_POSCFG_MAX':65535,'LPF_MAX_HZ':1000}
# Every plain release on each certified line. `pack()` in the Rust core resolves
# a dump from any of these to the pack certified at the first release of the
# line, so a default may only be recorded if it is identical at all of them.
PATCH_RELEASES = {'4.2.0':[f'4.2.{patch}' for patch in range(12)],
 '4.3.0':['4.3.0','4.3.1','4.3.2'],
 '4.4.0':['4.4.0','4.4.1','4.4.2','4.4.3'],
 '4.5.0':['4.5.0','4.5.1','4.5.2','4.5.3','4.5.4','4.5.5']}
RESET_FN = 'pgResetFn_controlRateProfiles'
def reset_body(text):
    """The one reset function whose defaults this generator certifies."""
    start = text.index('void ' + RESET_FN)
    return text[start:text.index('\n}', start)]

for version in PATCH_RELEASES:
    sources = {}
    def fetch(path):
        url = f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}'
        data = urllib.request.urlopen(url).read()
        sources[path] = {'url':url, 'sha256':hashlib.sha256(data).hexdigest()}
        return data.decode()
    settings = fetch('cli/settings.c')
    # 4.2 uses literal CLI names; parameter_names.h was added later.
    names = '' if version == '4.2.0' else fetch('fc/parameter_names.h')
    settings_h = fetch('cli/settings.h')
    common_pre = fetch('target/common_pre.h')
    defines = dict(re.findall(r'#define\s+(\w+)\s+"([^"\n]+)"', names))
    arrays = {n: re.findall(r'"([^"\n]+)"', body) for n,body in re.findall(r'(\w+)\[\]\s*=\s*\{(.*?)\};',settings,re.S)}
    table_names = re.findall(r'^\s*(TABLE_\w+)(?:\s*=\s*0)?\s*,', settings_h, re.M)
    table_arrays = re.findall(r'^\s*LOOKUP_TABLE_ENTRY\((\w+)\)', settings, re.M)
    assert len(table_names) == len(table_arrays), (version, len(table_names), len(table_arrays))
    lookups = {key: arrays[array] for key,array in zip(table_names,table_arrays) if array in arrays}
    params = {}
    for line in settings.splitlines():
        m = re.search(r'\{\s*("[^"]+"|PARAM_NAME_\w+)\s*,\s*VAR_(\w+)\s*\|\s*(MASTER_VALUE|PROFILE_RATE_VALUE|PROFILE_VALUE)',line)
        if not m: continue
        raw,ty,scope = m.groups()
        name = raw.strip('"') if raw.startswith('"') else defines.get(raw)
        if not name: continue
        # MODE_BITSET settings are single flag bits; the CLI prints and accepts
        # them as ON/OFF (cli.c), never as a number. Typing them as integers
        # makes every real dump fail schema validation.
        bitset = 'MODE_BITSET' in line
        kind = 'string' if 'MODE_STRING' in line else 'enum' if ('MODE_LOOKUP' in line or bitset) else 'array' if 'MODE_ARRAY' in line else 'integer'
        bounds = re.search(r'config.minmax(?:Unsigned)?\s*=\s*\{\s*(\w+)\s*,\s*(\w+)',line)
        def number(s): return int(s) if s.isdigit() else CONSTANTS.get(s)
        enum = re.search(r'config.lookup\s*=\s*\{\s*(\w+)',line)
        params[name] = {'scope':{'MASTER_VALUE':'global','PROFILE_RATE_VALUE':'rate','PROFILE_VALUE':'pid'}[scope],
            'kind':kind, 'min':number(bounds[1]) if bounds else None,'max':number(bounds[2]) if bounds else None,
            'values':['OFF','ON'] if bitset else (lookups.get(enum[1],[]) if enum else [])}
    # Profile counts are build-dependent: common_pre.h lowers them on
    # flash-constrained targets. A backup can only have been written by one
    # build, so the widest definition on the line is the bound that accepts
    # every legitimate dump.
    def count(macro):
        found = [int(v) for v in re.findall(rf'#define\s+{macro}\s+(\d+)', common_pre)]
        assert found, (version, macro)
        return max(found)
    profiles = {'pid':count('PID_PROFILE_COUNT'), 'rate':count('CONTROL_RATE_PROFILE_COUNT')}
    # A Betaflight diff prints only what differs from the reset the firmware
    # applies on `defaults`, so a key the dump never sets still holds the reset
    # value. That is readable evidence rather than a guess -- but only if the
    # reset is sourced as rigorously as the schema, so it is extracted here from
    # the same pinned tag and joined to the CLI key by settings.c's own
    # `offsetof`. Nothing about defaults is hand-written.
    #
    # Only PG_CONTROL_RATE_PROFILES is certified. Other parameter groups do move
    # between minor releases and each needs its own verification pass.
    rate_c = fetch('fc/controlrate_profile.c')
    rate_h = fetch('fc/controlrate_profile.h')
    pid_h = fetch('flight/pid.h')
    enums = {}
    for body in re.findall(r'typedef enum\s*\{(.*?)\}', rate_h + pid_h, re.S):
        nxt = 0
        for name, explicit in re.findall(r'(\w+)\s*(?:=\s*(\d+))?\s*(?:,|$)', body, re.M):
            nxt = int(explicit) if explicit else nxt
            enums.setdefault(name, nxt)
            nxt += 1
    fields = {field: name.strip('"') if name.startswith('"') else defines.get(name)
        for name, field in re.findall(
            r'\{\s*("[^"]+"|PARAM_NAME_\w+).*?PG_CONTROL_RATE_PROFILES,\s*offsetof\(controlRateConfig_t,\s*([^)]+)\)', settings)}
    body = reset_body(rate_c)
    defaults = {}
    for field, raw in re.findall(r'^\s*\.([\w\[\]]+)\s*=\s*([^,\n]+),\s*$', body, re.M):
        key = fields.get(field.strip())
        # An aggregate initialiser (`.profileName = { 0 }`) has no single CLI
        # value, and a struct field with no settings.c entry has no CLI name.
        if not key or raw.startswith('{'): continue
        schema = params.get(key)
        number = int(raw) if raw.lstrip('-').isdigit() else CONSTANTS.get(raw, enums.get(raw))
        if schema is None or number is None:
            print(f'{version}: skipping default for {key or field} ({raw})')
            continue
        if schema['kind'] == 'enum':
            # A lookup-table setting stores the enum ordinal and the CLI prints
            # the table entry at that index, so the pack's own value list is the
            # translation -- no second name mapping to drift out of date.
            if not 0 <= number < len(schema['values']):
                print(f'{version}: skipping default for {key} (index {number} outside lookup)')
                continue
            defaults[key] = schema['values'][number]
        elif schema['kind'] == 'integer':
            lo, hi = schema['min'], schema['max']
            assert (lo is None or number >= lo) and (hi is None or number <= hi), (version, key, number)
            defaults[key] = str(number)
        else:
            print(f'{version}: skipping default for {key} (kind {schema["kind"]})')
    assert defaults, version
    # `pack()` accepts any patch release on the line against this one pack. That
    # is safe for syntax and bounds regardless; for defaults it is only safe
    # while the reset itself does not move, so prove it at every release rather
    # than trusting the convention.
    reset = hashlib.sha256(body.encode()).hexdigest()
    verified = []
    for tag in PATCH_RELEASES[version]:
        url = f'https://raw.githubusercontent.com/betaflight/betaflight/{tag}/src/main/fc/controlrate_profile.c'
        data = urllib.request.urlopen(url).read()
        digest = hashlib.sha256(reset_body(data.decode()).encode()).hexdigest()
        assert digest == reset, f'{RESET_FN} differs at {tag}; defaults are no longer invariant on the {version} line'
        verified.append({'version':tag, 'url':url, 'sha256':hashlib.sha256(data).hexdigest()})

    pack = {'id':f'betaflight-{version}-schema-1', 'version':version, 'sources':sources, 'parameters':params,
      'profiles':profiles,
      'defaults':{'source':f'fc/controlrate_profile.c {RESET_FN}', 'reset_sha256':reset,
        'verified':verified, 'values':defaults},
      'baseline':None, 'notice':'Schema does not certify build features or defaults. GPL-3.0-or-later; derived from Betaflight.'}
    (OUT / f'betaflight-{version}.json').write_text(json.dumps(pack,indent=2,sort_keys=True)+'\n')
