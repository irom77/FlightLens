"""Verify older throttle CLI declarations and shim constants against reviewed baselines.

Developer-only public-source downloads; no backup data is read.
Run: python3 tools/verify_throttle_releases.py
"""
import pathlib,re,urllib.request,hashlib,json,concurrent.futures
root=pathlib.Path(__file__).resolve().parents[1]
tags=re.findall(r'\| (4\.[234]\.\d+) \| `([a-f0-9]{40})`',(root/'docs/throttle-curve-preview.md').read_text())
paths=['fc/controlrate_profile.h','cli/settings.c','fc/rc.c','rx/rx.h']
def fetch(item):
 v,c,p=item
 data=urllib.request.urlopen(f'https://raw.githubusercontent.com/betaflight/betaflight/{c}/src/main/{p}',timeout=30).read()
 return (v,p,data)
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
 sources={(v,p):data for v,p,data in pool.map(fetch,[(v,c,p) for v,c in tags for p in paths])}
result=[]
for v,c in tags:
 base='.'.join(v.split('.')[:2])+'.0'
 selections={}
 for p in paths:
  def select(data):
   text=data.decode()
   if p=='fc/controlrate_profile.h':
    return [line.strip() for line in text.splitlines() if re.search(r'thrMid8|thrExpo8|throttle_limit|THROTTLE_LIMIT_TYPE',line)]
   if p=='cli/settings.c':
    return [line.strip() for line in text.splitlines() if 'offsetof(controlRateConfig_t,' in line and re.search(r'thrMid8|thrExpo8|throttle_limit',line)] + re.findall(r'const char \* const lookupTableThrottleLimitType\[\] = \{.*?\};',text,re.S)
   if p=='fc/rc.c':
    return re.findall(r'#define THROTTLE_LOOKUP_LENGTH\s+\d+',text)
   return [line.strip() for line in text.splitlines() if re.search(r'#define\s+PWM_RANGE(?:_MIN|_MAX)?\s',line)]
  selected=select(sources[v,p]); assert selected,(v,p)
  assert selected==select(sources[base,p]),(v,p,selected)
  selections[p]={'sha256':hashlib.sha256(sources[v,p]).hexdigest(),'verifiedDeclarations':selected}
 result.append({'version':v,'commit':c,'matchesBaseline':base,'sources':selections})
(root/'fixtures/throttle-release-evidence.json').write_text(json.dumps(result,indent=2)+'\n')
print(f'{len(result)} releases: CLI declarations, profile fields/enums, lookup length and PWM constants match their line baseline')
