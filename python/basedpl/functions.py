"Word names and composition over the interpreter's immutable function nodes."
import builtins
from keyword import iskeyword
from unicodedata import normalize
from . import _Operators, _Function, _array, bpl, symbols
from ._core import _system_functions

_HOLE = object()

def _build(kind, *operands, valence=0):
    if any(o is _HOLE or isinstance(o, _Pending) for o in operands): return _Pending(kind, operands, valence)
    if kind == 'history':
        f, p = operands
        return _build('⍣', f, [p] if isinstance(p, Function) else bpl('{(×⍵)×⍳1+|⍵}')(p))
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
        return bpl._request(dict(source=self._inner, args=[_array(o) for o in args]), True).value

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

# Operator properties take Rust's operator names in Python spelling. Rust names `/⌿\⍀` after their shapes. `reduce` and `scan`
# spell `/` and `\`, and `f.reduce[0]` and `f.scan[0]` spell `⌿` and `⍀`. `f[axis]` spells `⍠`. Rust has no row for `⁻¹` or
# history. A dyadic operator waits for its right operand.
_operators = [(s['glyph'], _python_name(s['name']), s['kind']) for s in symbols if s['kind'].endswith('-operator') and s['glyph'] not in '/⌿\\⍀⍠']
_MONADIC = dict(reduce='/', scan='\\', undo='⁻¹') | {n: g for g, n, k in _operators if k == 'monadic-operator'}
_DYADIC = {n: g for g, n, k in _operators if k == 'dyadic-operator'} | dict(history='history')
for _name, _glyph in _MONADIC.items(): setattr(_Combinators, _name, property(lambda self, g=_glyph: _build(g, self)))
for _name, _glyph in _DYADIC.items(): setattr(_Combinators, _name, property(lambda self, g=_glyph: _build(g, self, _HOLE)))

_builtins = {alias: (name, 0) for name in _system_functions for alias in (name, name[1:])}
for _s in symbols:
    if _s['kind'] != 'function': continue
    for _name in (_s['glyph'], _s['name'], *_s['aliases'].split()):
        _builtins[_name] = _builtins[_python_name(_name)] = (_s['glyph'], 0)
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

def _binary(glyph, x, y):
    f = _builtin(glyph)
    return fork(x, f, y) if isinstance(x, _Combinators) or isinstance(y, _Combinators) else f(x, y)

def _unary(glyph, y):
    f = _builtin(glyph)
    return atop(f, y) if isinstance(y, _Combinators) else f(y)
