"Word names and composition over the interpreter's immutable function nodes."
from keyword import iskeyword
from . import _Operators, _Function, _array, _context, Session, symbols

_default_session = None
_HOLE = object()

def _default():
    global _default_session
    if _default_session is None: _default_session = Session()
    return _default_session

def _build(kind, *operands, valence=0):
    if any(o is _HOLE or isinstance(o, _Pending) for o in operands): return _Pending(kind, operands, valence)
    if kind == 'history': return _build('⍣', operands[0], enclose(operands[1]))
    session, values = _context(*operands), []
    for o in operands:
        if isinstance(o, Function): values.append(o._inner)
        else: values.append(_array(o))
    return Function(_Function.build(kind, values), valence=valence, session=session)

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
    "An APL function node, optionally associated with a session for name lookup."
    def __init__(self, inner, valence=0, session=None): self._inner, self._valence, self._session = inner, valence, session
    def __repr__(self): return repr(self._inner)
    def __call__(self, *args):
        if len(args) == 1 and self._valence == 2: return _build('∘', self, args[0], valence=1)
        if len(args) not in (1, 2) or len(args) == 2 and self._valence == 1: raise TypeError('wrong number of arguments for this APL function')
        owner = _context(self, *args)
        session = owner if owner is not None else _default()
        return session._request(dict(function=self._inner, args=[_array(o) for o in args]), True).value

def fork(f, g, h): return _build('fork', f, g, h)
def atop(f, g): return _build('⍤', f, g)

__all__ = ['Function', 'fork', 'atop']
_primitives = {}
for _glyph, _glyph_name, _monad, _dyad, _aliases in symbols:
    if not (_monad or _dyad): continue
    _inner = _Function.builtin(_glyph)
    _primitives[_glyph] = Function(_inner)
    for _valence, _name in enumerate((_monad, _dyad), 1):
        if _name:
            _name = _name.replace('-', '_')
            if iskeyword(_name): _name += '_'
            globals()[_name] = Function(_inner, _valence)
            __all__.append(_name)

def _binary(glyph, x, y):
    f = _primitives[glyph]
    return fork(x, f, y) if isinstance(x, _Combinators) or isinstance(y, _Combinators) else f(x, y)

def _unary(glyph, y):
    f = _primitives[glyph]
    return atop(f, y) if isinstance(y, _Combinators) else f(y)
