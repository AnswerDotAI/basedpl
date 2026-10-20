"Word names and composition over the interpreter's immutable function nodes."
import builtins
from keyword import iskeyword
from unicodedata import normalize
from . import Array, _Operators, _Function, _array, _show, bpl, symbols
from ._core import _system_functions

_HOLE = object()

def _arguments(args): return [_array(o) for o in args], [type(o) in (int, float, complex) for o in args]

def _build(kind, *operands, valence=0):
    if any(o is _HOLE or isinstance(o, _Pending) for o in operands): return _Pending(kind, operands, valence)
    values = [o._inner if isinstance(o, Function) else _array(o) for o in operands]
    return Function(_Function.build(kind, values), valence=valence)

class _Combinators(_Operators):
    def __bool__(self): raise TypeError('a BPL function has no truth value')
    @property
    def left(self): return _build('↣', _HOLE, self, valence=1)
    def __pow__(self, counts): return _build('⍣', self, counts)
    def __getitem__(self, axis): return _build('axis', self, axis, valence=self._valence)
    def __lshift__(self, g): return _build('∘', self, g)
    def __rshift__(self, g): return _build('∘', g, self)
    def __matmul__(self, g):
        if not isinstance(g, _Combinators): raise TypeError('inner product requires two functions')
        return _build('.', self, g)
    def __rmatmul__(self, g): raise TypeError('inner product requires two functions')

class _Pending(_Combinators):
    "Function construction awaiting operator operands, innermost first."
    def __init__(self, kind, operands, valence): self.kind, self.operands, self._valence = kind, operands, valence
    def __repr__(self): return self.kind + '(' + ', '.join('□' if o is _HOLE else repr(o) for o in self.operands) + ')'
    def __call__(self, operand):
        values = list(self.operands)
        for i,o in enumerate(values):
            if isinstance(o, _Pending):
                values[i] = o(operand)
                break
        else:
            i = next(i for i,o in enumerate(values) if o is _HOLE)
            values[i] = operand
        return _build(self.kind, *values, valence=self._valence)

class _Help:
    "Gives the class its docstring, and each function its own help."
    def __init__(self, doc): self.doc = doc
    def __get__(self, obj, cls=None): return self.doc if obj is None else obj.inspect()['help']

class Function(_Combinators):
    "A BPL function node."
    def __init__(self, inner, valence=0): self._inner, self._valence = inner, valence
    def __repr__(self): return repr(self._inner)
    def inspect(self):
        "BPL source and help, without running the function."
        info = bpl._session.inspect(function=self._inner)
        calls = ('f(right) or f(left, right)', 'f(right)', 'f(left, right); f(right) binds the right argument')[self._valence]
        title, _, rest = info['help'].partition('\n')
        info['help'] = f"{title.removeprefix('# ')}\n\nCalls: {calls}\n{rest}"
        return info
    @property
    def source(self): return self.inspect()['source']
    __doc__ = _Help(__doc__)
    def __call__(self, *args, **kwargs):
        "Call with `⍵` or `⍺, ⍵`. Keyword arguments supply `⍺` as a keyed vector, and the positional arguments then form `⍵`."
        if kwargs:
            if self._valence == 1: raise TypeError('keyword arguments need a dyadic call')
            args = (kwargs, args[0] if len(args) == 1 else list(args))
        elif len(args) == 1 and self._valence == 2: return _build('↢', self, args[0], valence=1)
        if len(args) not in (1, 2) or len(args) == 2 and self._valence == 1: raise TypeError('wrong number of arguments for this BPL function')
        return self._invoke(*_arguments(args))
    def __rrshift__(self, value): return self(value)
    def _invoke(self, args, weak): return bpl._call(self._inner, args, _show, weak).value

class _Attaching(Function):
    "`f.undo` or `f.derivative`. Called with one function `g`, it attaches `g` to `f` as its inverse or derivative, as `f⁻¹←g` and `f∂←g` do in BPL."
    def __init__(self, glyph, f, attach, valence):
        super().__init__(_build(glyph, f)._inner, valence)
        self._f, self._attach = f, attach
    def __call__(self, *args, **kwargs):
        if len(args) == 1 and not kwargs and isinstance(args[0], Function): return Function(getattr(self._f._inner, self._attach)(args[0]._inner))
        return super().__call__(*args, **kwargs)

class Operator:
    "A BPL operator held as a value, as a module record holds it. Call it with its operand, or both operands of a dyadic operator, to get a `Function`."
    def __init__(self, inner): self._inner = inner
    def __repr__(self): return repr(self._inner)
    def __call__(self, *operands): return Function(self._inner.derive(*[o._inner if isinstance(o, Function) else _array(o) for o in operands]))

def fork(f, g, h): return _build('fork', f, g, h)
def atop(f, g): return _build('∘', f, g)

def _python_name(name):
    name = normalize('NFKC', name.replace('-', '_'))
    return name+'_' if iskeyword(name) else name

# Every documented name of an operator is a property, in Python spelling. Its glyph name gives the derived function with both
# valences, and each name for what it does gives the valences that name covers. A dyadic operator's property waits for its right
# operand. `f[axis]` also spells `⍠`, and `f.undo` spells `⁻¹`, which has no row. `f.undo(g)` and `f.derivative(g)` with a function
# `g` attach it, as `f⁻¹←g` and `f∂←g` do in BPL.
_operators = {'undo': ('⁻¹', False, 0)}  # Python name: glyph, whether the operator is dyadic, and the derived function's valence
_operator_names = {}  # An operator's glyph: its glyph name, then its names with one argument and with two, in Python spelling
for _s in symbols:
    if not _s['kind'].endswith('-operator'): continue
    _glyph, _dyadic = _s['glyph'], _s['kind'] == 'dyadic-operator'
    _operator_names[_glyph] = tuple(map(_python_name, (_s['name'], _s['monad'], _s['dyad'] or _s['monad'])))
    _operators[_python_name(_s['name'])] = (_glyph, _dyadic, 0)
    for _valence, _name in enumerate((_s['monad'], _s['dyad']), 1):
        if _name: _operators[_python_name(_name)] = (_glyph, _dyadic, _valence if _s['dyad'] else 0)
_ATTACHING = dict(undo='with_inverse', derivative='with_derivative')

def _operator(name, glyph, dyadic, valence):
    if name in _ATTACHING:
        return property(lambda self: _Attaching(glyph, self, _ATTACHING[name], valence) if isinstance(self, Function) else _build(glyph, self, valence=valence))
    if dyadic: return property(lambda self: _build(glyph, self, _HOLE, valence=valence))
    return property(lambda self: _build(glyph, self, valence=valence))
for _name, _spec in _operators.items(): setattr(_Combinators, _name, _operator(_name, *_spec))

_builtins = {alias: (name, 0) for name in _system_functions for alias in (name, name[1:])}
for _s in symbols:
    if _s['kind'] != 'function': continue
    for _name in (_s['glyph'], _s['name']): _builtins[_name] = _builtins[_python_name(_name)] = (_s['glyph'], 0)
    for _valence, _name in enumerate((_s['monad'], _s['dyad']), 1):
        if _name: _builtins[_name] = _builtins[_python_name(_name)] = (_s['glyph'], _valence)

__all__ = ['Function', 'Operator', 'fork', 'atop', *(name for name in _builtins if name.isidentifier() and not iskeyword(name) and not hasattr(builtins, name))]

def _builtin(name):
    "Resolve a builtin with operation-name valence."
    source, valence = _builtins.get(name, (name if name.startswith('•') else '•'+name, 0))
    try: inner = _Function.builtin(source)
    except ValueError: raise AttributeError(f'unknown builtin function: {name!r}') from None
    return Function(inner, valence)

def __getattr__(name): return _builtin(name)
def __dir__(): return sorted(set(globals()) | _builtins.keys())

def _glyph(glyph):
    "The primitive function `glyph`, with both valences."
    return Function(_Function.builtin(glyph))

def _binary(f, x, y):
    if isinstance(f, str): f = _glyph(f)
    composed = isinstance(x, _Combinators) or isinstance(y, _Combinators)
    try:
        if composed: operands = [o if isinstance(o, _Combinators) else _array(o) for o in (x, y)]
        else: args = _arguments((x, y))
    except TypeError: return NotImplemented
    return fork(operands[0], f, operands[1]) if composed else f._invoke(*args)

def _unary(f, y):
    if y is NotImplemented: return NotImplemented
    if isinstance(f, str): f = _glyph(f)
    return atop(f, y) if isinstance(y, _Combinators) else f(y)

def _bitwise(glyph, *operands):
    try: operands = [o if isinstance(o, (Array, _Combinators)) else Array(o) for o in operands]
    except TypeError: return NotImplemented
    if any(isinstance(o, Array) and not o._inner.is_integral for o in operands): raise TypeError('bitwise operations require integers or Booleans')
    f = _glyph(glyph).bitwise
    return _unary(f, *operands) if len(operands) == 1 else _binary(f, *operands)

def _matmul(x, y):
    if isinstance(x, _Combinators) or isinstance(y, _Combinators): raise TypeError('inner product requires two arrays or two functions')
    try: x, y = (o if isinstance(o, Array) else Array(o) for o in (x, y))
    except TypeError: return NotImplemented
    left, right = x.shape, y.shape
    if not left or not right: raise ValueError('matrix multiplication requires rank at least one')
    if left[-1] != right[-min(2, len(right))]: raise ValueError('matrix multiplication contraction lengths must agree')
    return _glyph('+').inner_product(_glyph('×')).rank(2)(x, y)
