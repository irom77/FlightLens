"""Resolve audited CLI bound expressions without executing upstream C or Python."""
import ast
import operator
import re

# These headers supply the non-hardware bounds used by the 2025.12 schema.
BOUND_HEADERS = '''common/color.h common/time.h config/simplified_tuning.h
 drivers/display.h drivers/motor_types.h drivers/vtx_common.h drivers/vtx_table.h drivers/dshot_command.h
 fc/core.h fc/rc.h fc/rc_controls.h fc/controlrate_profile.h
 flight/dyn_notch_filter.h flight/failsafe.h flight/pid.h osd/osd.h
 pg/stats.h rx/crsf.h rx/cyrf6936_spektrum.h rx/rx.h rx/spektrum.h
 sensors/battery.h sensors/gyro.h sensors/voltage.h
 telemetry/frsky_hub.h telemetry/telemetry.h'''.split()

OPS = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul,
       ast.Div: lambda a, b: int(a / b), ast.LShift: operator.lshift,
       ast.BitOr: operator.or_}


def resolver(texts):
    definitions = {}
    for text in texts:
        text = re.sub(r'/\*.*?\*/|//[^\n]*', '', text, flags=re.S)
        for name, expression in re.findall(r'^\s*#define\s+(\w+)[ \t]+([^\n]+)', text, re.M):
            definitions.setdefault(name, set()).add(expression.strip())
        for body in re.findall(r'\benum\s*\{([^}]+)\}', text):
            # Conditional ordinals cannot be inferred without a target build.
            if '#' in body:
                continue
            previous = '-1'
            for entry in body.split(','):
                entry = entry.strip()
                if not entry:
                    continue
                match = re.fullmatch(r'(\w+)\s*(?:=\s*(.*))?', entry)
                if not match:
                    break
                name, explicit = match.groups()
                definitions.setdefault(name, set()).add(explicit or f'({previous}) + 1')
                previous = name
    for name, value in {'UINT8_MAX': 255, 'UINT16_MAX': 65535, 'UINT32_MAX': 4294967295,
                        'INT8_MIN': -128, 'INT8_MAX': 127,
                        'INT16_MIN': -32768, 'INT16_MAX': 32767}.items():
        definitions[name] = {str(value)}

    def resolve(expression, seen=frozenset()):
        def visit(node):
            if isinstance(node, ast.Constant) and type(node.value) is int:
                return node.value
            if isinstance(node, ast.Name) and node.id not in seen:
                choices = definitions.get(node.id, set())
                if len(choices) == 1:
                    return resolve(next(iter(choices)), seen | {node.id})
            if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.USub, ast.UAdd)):
                value = visit(node.operand)
                return -value if isinstance(node.op, ast.USub) else value
            if isinstance(node, ast.BinOp) and type(node.op) in OPS:
                return OPS[type(node.op)](visit(node.left), visit(node.right))
            raise ValueError(f'Unresolved bound: {expression}')
        # C integer suffixes do not change these small integer bounds.
        expression = re.sub(r'\b(\d+)[uUlL]+\b', r'\1', expression.strip())
        return visit(ast.parse(expression, mode='eval').body)
    return resolve
