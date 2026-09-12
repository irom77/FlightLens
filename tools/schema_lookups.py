"""Extract lookup tables without losing choices in conditional definitions."""
import re


def lookup_arrays(source, *, sized=False):
    size = r'[^]\n]*' if sized else ''
    arrays = {}
    for name, body in re.findall(r'(\w+)\[' + size + r'\]\s*=\s*\{(.*?)\};', source, re.S):
        values = re.findall(r'"([^"\n]+)"', body)
        previous = arrays.get(name, [])
        # A conditional shorter table may omit trailing feature choices. Keep
        # the widest table only when shared ordinals agree; defaults use them.
        shared = min(len(previous), len(values))
        if previous[:shared] != values[:shared]:
            raise ValueError(f'Conflicting conditional lookup ordinals: {name}')
        arrays[name] = values if len(values) > len(previous) else previous
    return arrays
