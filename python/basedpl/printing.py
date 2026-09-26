"Informative Python spelling of native function structure."
from fractions import Fraction
from itertools import repeat
from json import dumps
from . import _Array, _Function, symbols
from .functions import Function, _builtins, _python_name

_names = {g: (m, d) for g, _, m, d, *_ in symbols}
_infix = {'+': ('+', 20), '-': ('-', 20), '×': ('*', 30), '÷': ('/', 30),
          '=': ('==', 10), '≠': ('!=', 10), '<': ('<', 10), '≤': ('<=', 10), '>': ('>', 10), '≥': ('>=', 10)}

def _apl(f): return f'fn({dumps(repr(f), ensure_ascii=False)})'

def _atom(o):
    if isinstance(o, tuple): return repr(Fraction(*o))
    if isinstance(o, float):
        if abs(o) == float('inf'): return "float('-inf')" if o < 0 else "float('inf')"
        text = repr(o)
        return text[:-1] if text.endswith('.0') else text
    return repr(o)

def _literal(a):
    raw = a.parts()
    if 'atom' in raw:
        o = raw['atom']
        return _apl(o) if isinstance(o, _Function) else _atom(o)
    if len(raw['shape']) == 1 and raw['data'] and not any(k in raw for k in ('axis_keys', 'axis_names')):
        if all(isinstance(o, str) for o in raw['data']): return repr(''.join(raw['data']))
        if all(isinstance(o, (int, float, complex, tuple)) for o in raw['data']):
            return '[' + ', '.join(_atom(o) for o in raw['data']) + ']'
    return f'apl({dumps(repr(a), ensure_ascii=False)})'

class _Printer:
    def __init__(self): self.budget = repeat(True, 1000)
    def render(self, f, dyad=False, context=0):
        if isinstance(f, _Array): return _literal(f)
        if not next(self.budget, False): return '…'
        text, precedence = self.expression(f, dyad)
        return f'({text})' if precedence < context else text

    def expression(self, f, dyad):
        parts = f.parts()
        if parts is None: return _apl(f), 100
        kind, args = parts
        r = self.render
        if not args:
            name = _names.get(kind, ('', ''))[dyad]
            if name: return _python_name(name), 100
            name = _python_name(kind.removeprefix('•'))
            if _builtins.get(name, (None,))[0] == kind: return name, 100
            return _apl(f), 100
        a = args[0]
        if kind in ('/', '⌿', '\\', '⍀'):
            method = 'scan' if kind in ('\\', '⍀') else 'reduce'
            axis = f'[{r(args[1])}]' if len(args) == 2 else '[0]' if kind in ('⌿', '⍀') else ''
            return f'{r(a, True, 100)}.{method}{axis}', 100
        if kind == 'axis': return f'{r(a, dyad, 100)}[{r(args[1])}]', 100
        if kind == 'inverse': return f'{r(a, dyad, 41)} ** -1', 40
        if kind == 'fork':
            b, c = args[1:]
            middle = b.parts()
            if middle and not middle[1] and middle[0] in _infix:
                op, p = _infix[middle[0]]
                return f'{r(a, dyad, p+1)} {op} {r(c, dyad, p+1)}', p
            return f'fork({r(a, dyad)}, {r(b, True)}, {r(c, dyad)})', 100
        if kind in ('¨', '⍨', '⌝', '⌸', '∂') and isinstance(a, _Function):
            method = {'¨': 'each', '⍨': 'commute', '⌝': 'outer', '⌸': 'key', '∂': 'derivative'}[kind]
            valence = dyad if kind == '¨' else kind != '∂'
            return f'{r(a, valence, 100)}.{method}', 100
        if len(args) != 2: return _apl(f), 100
        b = args[1]
        af, bf = isinstance(a, _Function), isinstance(b, _Function)
        if kind == '⊸' and not af: return f'{r(b, True, 100)}.left({r(a)})', 100
        if kind == '⟜' and not bf:
            operand = a.parts()
            if operand and not operand[1] and _names.get(operand[0], ('', ''))[1]:
                return f'{r(a, True, 100)}({r(b)})', 100
            return f'{r(a, True, 100)}.after({r(b)})', 100
        if kind == '.': return f'{r(a, True, 31)} @ {r(b, True, 31)}', 30
        if kind == '⍣':
            if bf: return f'{r(a, dyad, 100)}.power({r(b, True)})', 100
            raw = b.parts()
            if raw.get('shape') == [1] and isinstance(raw['data'][0], _Function): return f'{r(a, dyad, 100)}.history({r(raw["data"][0], True)})', 100
            return f'{r(a, dyad, 41)} ** {r(b)}', 40
        methods = {'⍤': ('atop' if bf else 'rank', False if bf else dyad, dyad),
                   '⍥': ('over', dyad, False), '⊸': ('before', False, True), '⟜': ('after', True, False), '⌾': ('under', dyad, False),
                   '⇄': ('with_inverse', dyad, dyad), '@': ('at', dyad, False), '⌺': ('stencil', True, False)}
        if kind in methods and af:
            method, av, bv = methods[kind]
            return f'{r(a, av, 100)}.{method}({r(b, bv)})', 100
        return _apl(f), 100

def to_python(f, dyad=False):
    "Describe a function using Python names; choose dyadic spelling for an ambivalent root with dyad=True."
    if not isinstance(f, Function): raise TypeError('to_python expects a Function')
    return _Printer().render(f._inner, f._valence == 2 if f._valence else dyad)
