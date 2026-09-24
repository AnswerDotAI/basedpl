"Word names and composition over the interpreter's immutable function nodes."
from keyword import iskeyword
from unicodedata import normalize
from . import _Operators, _Function, _array, apl, symbols
from ._core import _system_functions

_HOLE = object()

def _build(kind, *operands, valence=0):
    if any(o is _HOLE or isinstance(o, _Pending) for o in operands): return _Pending(kind, operands, valence)
    if kind == 'history': return _build('⍣', operands[0], _builtin('⊂')(operands[1]))
    values = [o._inner if isinstance(o, Function) else _array(o) for o in operands]
    return Function(_Function.build(kind, values), valence=valence)

class _Combinators(_Operators):
    def __bool__(self): raise TypeError('an APL function has no truth value')
    @property
    def left(self): return _build('∘', _HOLE, self, valence=1)
    @property
    def reduce(self): return _build('/', self)
    @property
    def scan(self): return _build('\\', self)
    @property
    def each(self): return _build('¨', self)
    @property
    def commute(self): return _build('⍨', self)
    @property
    def outer(self): return _build('⌝', self)
    @property
    def key(self): return _build('⌸', self)
    @property
    def inner(self): return _build('.', self, _HOLE)
    @property
    def rank(self): return _build('⍤', self, _HOLE)
    @property
    def beside(self): return _build('∘', self, _HOLE)
    @property
    def atop(self): return _build('⍤', self, _HOLE)
    @property
    def over(self): return _build('⍥', self, _HOLE)
    @property
    def behind(self): return _build('⍛', self, _HOLE)
    @property
    def under(self): return _build('⌾', self, _HOLE)
    @property
    def with_inverse(self): return _build('⇄', self, _HOLE)
    @property
    def power(self): return _build('⍣', self, _HOLE)
    @property
    def history(self): return _build('history', self, _HOLE)
    @property
    def derivative(self): return _build('∂', self)
    @property
    def at(self): return _build('@', self, _HOLE)
    @property
    def stencil(self): return _build('⌺', self, _HOLE)
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

class Function(_Combinators):
    "An APL function node."
    def __init__(self, inner, valence=0): self._inner, self._valence = inner, valence
    def __repr__(self): return repr(self._inner)
    def inspect(self):
        "APL source and help, without running the function."
        info = apl._session.inspect(function=self._inner)
        calls = ('f(right) or f(left, right)', 'f(right)', 'f(left, right); f(right) binds the right argument')[self._valence]
        info['help'] = f'Calls: {calls}\n\n' + info['help']
        return info
    @property
    def source(self): return self.inspect()['source']
    @property
    def __doc__(self): return self.inspect()['help']
    def __call__(self, *args, **kwargs):
        "Call with `⍵` or `⍺, ⍵`. Keyword arguments supply `⍺` as a keyed vector, and the positional arguments then form `⍵`."
        if kwargs:
            if self._valence == 1: raise TypeError('keyword arguments need a dyadic call')
            args = (kwargs, args[0] if len(args) == 1 else list(args))
        elif len(args) == 1 and self._valence == 2: return _build('∘', self, args[0], valence=1)
        if len(args) not in (1, 2) or len(args) == 2 and self._valence == 1: raise TypeError('wrong number of arguments for this APL function')
        return apl._request(dict(function=self._inner, args=[_array(o) for o in args]), True).value

def fork(f, g, h): return _build('fork', f, g, h)
def atop(f, g): return _build('⍤', f, g)

def _python_name(name):
    name = normalize('NFKC', name.replace('-', '_'))
    return name+'_' if iskeyword(name) else name

_builtins = {alias: (name, 0) for name in _system_functions for alias in (name, name[1:])}
for _glyph, _glyph_name, _monad, _dyad, _aliases, _shortcut in symbols:
    if not (_monad or _dyad): continue
    for _name in (_glyph, _glyph_name, *_aliases.split()):
        if not _name: continue
        _builtins[_name] = _builtins[_python_name(_name)] = (_glyph, 0)
    for _valence, _name in enumerate((_monad, _dyad), 1):
        if _name: _builtins[_name] = _builtins[_python_name(_name)] = (_glyph, _valence)

__all__ = ['Function', 'fork', 'atop', *(name for name in _builtins if name.isidentifier() and not iskeyword(name))]

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
