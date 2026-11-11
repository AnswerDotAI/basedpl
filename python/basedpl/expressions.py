"Python parts for BPL's expression parser."
import operator
from fractions import Fraction
from . import Array, _array
from .functions import Function, Operator, _operator_names, _marks, __all__ as _function_names


class _Syntax:
    def __init__(self, source): self.source = source
    def __repr__(self): return self.source

class _Script(_Syntax): pass

space, dot, alpha, omega, diamond, guard, otherwise = map(_Syntax, (' ', '.', '⍺', '⍵', '⋄', '?', ';'))

def _script(values, digits):
    if not values: raise TypeError('a script needs at least one integer')
    return _Script(' '.join(str(operator.index(n)).translate(str.maketrans('-0123456789', digits)) for n in values))

def superscript(*ranks):
    "A numeric superscript: a power on data, Rank on a function."
    return _script(ranks, '⁻⁰¹²³⁴⁵⁶⁷⁸⁹')

def subscript(*indices):
    "A numeric subscript: positions on data, axes on a function."
    return _script(indices, '₋₀₁₂₃₄₅₆₇₈₉')

__all__ = ['space', 'dot', 'alpha', 'omega', 'diamond', 'guard', 'otherwise', 'superscript', 'subscript']
_syntax_names = {name:glyph for glyph, (_, *names) in _operator_names.items() for name in names if name}
_syntax_names.update({name:glyph for glyph,name in _marks.items()})
for _name, _glyph in _syntax_names.items():
    if _name not in _function_names and _name not in globals():
        globals()[_name] = _Syntax(_glyph)
        __all__.append(_name)


def _expression(parts):
    source, embedded, formal = [], [], False
    offset = 0

    def text(s):
        nonlocal offset
        source.append(s)
        offset += len(s.encode())

    def sequence(values, separator):
        for i,value in enumerate(values):
            if i and not isinstance(value, _Script): text(separator)
            part(value)

    def part(value):
        nonlocal formal
        if isinstance(value, _Syntax):
            formal |= value.source in ('⍺', '⍵')
            text(value.source)
        elif isinstance(value, (list, tuple)):
            opening, closing, separator = ('[', ']', ' ') if isinstance(value, list) else ('(', ')', '⁝')
            text(opening)
            sequence(value, separator)
            text(closing)
        elif isinstance(value, (Function, Operator, Array)):
            embedded.append((offset, offset + 3, value._inner, False))
            text('_py')
        elif isinstance(value, (bool, int, float, complex, str, Fraction)):
            native = _array(value)
            spelling = native.literal()
            embedded.append((offset, offset + len(spelling.encode()), native, True))
            text(spelling)
        else:
            embedded.append((offset, offset + 3, _array(value), False))
            text('_py')

    sequence(parts, '⁝')
    result = ''.join(source)
    if formal: return '{' + result + '}', [(start + 1, end + 1, value, literal) for start,end,value,literal in embedded]
    return result, embedded
