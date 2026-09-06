"""Explicit developer-only pack generation from pinned upstream tags; never runs in-app."""
import hashlib, json, pathlib, re, urllib.request
ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / 'crates/flightlens-core/compatibility'
OUT.mkdir(parents=True, exist_ok=True)
CONSTANTS = {'UINT8_MAX':255,'UINT16_MAX':65535,'INT8_MIN':-128,'INT8_MAX':127,
 'VTX_TABLE_MAX_BANDS':8,'VTX_TABLE_MAX_CHANNELS':8,'VTX_TABLE_MAX_POWER_LEVELS':8,'VTX_TABLE_MIN_USER_FREQ':5000,'VTX_TABLE_MAX_USER_FREQ':5999,'PID_GAIN_MAX':250,'CONTROL_RATE_CONFIG_RC_RATES_MAX':255,'CONTROL_RATE_CONFIG_RC_EXPO_MAX':100,
 'CONTROL_RATE_CONFIG_RATE_MAX':255,'CONTROL_RATE_CONFIG_RATE_LIMIT_MIN':200,
 'CONTROL_RATE_CONFIG_RATE_LIMIT_MAX':1998,'OSD_POSCFG_MAX':65535,'LPF_MAX_HZ':1000}
for version in ['4.3.0','4.4.0','4.5.0']:
    sources = {}
    def fetch(path):
        url = f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/{path}'
        data = urllib.request.urlopen(url).read()
        sources[path] = {'url':url, 'sha256':hashlib.sha256(data).hexdigest()}
        return data.decode()
    settings = fetch('cli/settings.c')
    names = fetch('fc/parameter_names.h')
    settings_h = fetch('cli/settings.h')
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
        kind = 'string' if 'MODE_STRING' in line else 'enum' if 'MODE_LOOKUP' in line else 'array' if 'MODE_ARRAY' in line else 'integer'
        bounds = re.search(r'config.minmax(?:Unsigned)?\s*=\s*\{\s*(\w+)\s*,\s*(\w+)',line)
        def number(s): return int(s) if s.isdigit() else CONSTANTS.get(s)
        enum = re.search(r'config.lookup\s*=\s*\{\s*(\w+)',line)
        params[name] = {'scope':{'MASTER_VALUE':'global','PROFILE_RATE_VALUE':'rate','PROFILE_VALUE':'pid'}[scope],
            'kind':kind, 'min':number(bounds[1]) if bounds else None,'max':number(bounds[2]) if bounds else None,
            'values':lookups.get(enum[1],[]) if enum else []}
    pack = {'id':f'betaflight-{version}-schema-1', 'version':version, 'sources':sources, 'parameters':params,
      'baseline':None, 'notice':'Schema does not certify build features or defaults. GPL-3.0-or-later; derived from Betaflight.'}
    (OUT / f'betaflight-{version}.json').write_text(json.dumps(pack,indent=2,sort_keys=True)+'\n')
