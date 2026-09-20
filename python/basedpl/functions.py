"Word names and composition over the interpreter's immutable function nodes."
from . import _Operators, _Function, _array, _context, Session

_default_session = None

def _default():
    global _default_session
    if _default_session is None: _default_session = Session()
    return _default_session

def _build(kind, *operands, valence=0):
    session, values = _context(*operands), []
    for o in operands:
        if isinstance(o, Function): values.append(o._inner)
        else: values.append(_array(o))
    return Function(_Function.build(kind, values), valence=valence, session=session)

class Function(_Operators):
    "An APL function node, optionally associated with a session for name lookup."
    def __init__(self, inner, valence=0, session=None): self._inner, self._valence, self._session = inner, valence, session
    def __repr__(self): return repr(self._inner)
    def __bool__(self): raise TypeError('an APL function has no truth value')
    def __call__(self, *args):
        if len(args) == 1 and self._valence == 2: return _build('∘', self, args[0], valence=1)
        if len(args) not in (1, 2) or len(args) == 2 and self._valence == 1: raise TypeError('wrong number of arguments for this APL function')
        owner = _context(self, *args)
        session = owner if owner is not None else _default()
        return session._request(dict(function=self._inner, args=[_array(o) for o in args]), True).value
    def left(self, x): return _build('∘', x, self, valence=1)
    def reduce(self): return _build('/', self)
    def scan(self): return _build('\\', self)
    def each(self): return _build('¨', self)
    def commute(self): return _build('⍨', self)
    def outer(self): return _build('⌝', self)
    def key(self): return _build('⌸', self)
    def inner(self, g): return _build('.', self, g)
    def rank(self, ranks): return _build('⍤', self, ranks)
    def beside(self, g): return _build('∘', self, g)
    def atop(self, g): return _build('⍤', self, g)
    def over(self, g): return _build('⍥', self, g)
    def behind(self, g): return _build('⍛', self, g)
    def under(self, g): return _build('⌾', self, g)
    def with_inverse(self, g): return _build('⇄', self, g)
    def power(self, counts): return _build('⍣', self, counts)
    def history(self, count_or_predicate): return self.power(enclose(count_or_predicate))
    def derivative(self): return _build('∂', self)
    def at(self, indices): return _build('@', self, indices)
    def stencil(self, spec): return _build('⌺', self, spec)
    def __pow__(self, counts): return self.power(counts)
    def __getitem__(self, axis): return _build('axis', self, axis, valence=self._valence)
    def __lshift__(self, g): return self.beside(g)
    def __rshift__(self, g): return _build('∘', g, self)
    def __matmul__(self, g):
        if not isinstance(g, Function): raise TypeError('inner product requires two functions')
        return self.inner(g)
    def __rmatmul__(self, g): raise TypeError('inner product requires two functions')

def fork(f, g, h): return _build('fork', f, g, h)
def atop(f, g): return _build('⍤', f, g)

# Each row names the monadic and dyadic valences of one primitive.
_vocabulary = [('+', 'conjugate', 'plus'), ('-', 'negate', 'subtract'), ('×', 'sign', 'times'), ('÷', 'reciprocal', 'divide'),
    ('*', 'exponential', 'power'), ('⍟', 'logarithm', 'log'), ('○', 'cis', 'circle'), ('π', 'pi_times', 'pi_ratio'), ('√', 'sqrt', 'root'),
    ('!', 'factorial', 'binomial'), ('|', 'magnitude', 'residue'), ('⌊', 'floor', 'min'), ('⌈', 'ceiling', 'max'), ('∧', 'polar', 'lcm'),
    ('∨', 'real_imag', 'gcd'), ('⍲', 'square', 'nand'), ('⍱', 'double', 'nor'), ('~', 'not_', 'without'), ('=', 'classify', 'equal'),
    ('≠', 'unique_mask', 'not_equal'), ('<', None, 'less'), ('≤', 'decrement', 'less_equal'), ('>', None, 'greater'),
    ('≥', 'increment', 'greater_equal'), ('≡', 'depth', 'match'), ('≢', 'tally', 'not_match'), ('⍳', 'iota', 'index_of'),
    ('⍸', 'where', 'interval_index'), ('∊', 'enlist', 'member'), ('∪', 'unique', 'union'), ('∩', None, 'intersection'), ('⍷', None, 'find'),
    ('⍋', 'grade_up', 'grade_up_by'), ('⍒', 'grade_down', 'grade_down_by'), ('⌷', 'materialise', 'index'), ('⊤', 'binary_encode', 'encode'),
    ('⊥', 'binary_decode', 'decode'), ('⌹', 'inverse', 'matrix_divide'), ('⍎', 'execute', None), ('⍕', 'format', 'format_spec'),
    ('⍴', 'shape', 'reshape'), (',', 'ravel', 'catenate'), ('⍪', 'table', 'catenate_first'), ('⊂', 'enclose', 'partitioned_enclose'),
    ('⊆', 'nest', 'partition'), ('⊃', 'mix', 'pick'), ('↑', 'first', 'take'), ('↓', 'split', 'drop'), ('⌽', 'reverse', 'rotate'),
    ('⊖', 'reverse_first', 'rotate_first'), ('⍉', 'transpose', 'reorder_axes'), ('?', 'roll', 'deal'), ('/', None, 'replicate'),
    ('⌿', None, 'replicate_first'), ('\\', None, 'expand'), ('⍀', None, 'expand_first'), ('⊣', 'same_left', 'left'), ('⊢', 'same', 'right'),
    ('•C', 'case_fold', 'case_convert'), ('•UCS', 'unicode', 'unicode_convert'), ('↕', None, 'windows'), ('ℙ', 'prime', 'prime_mode'),
    ('Ⓠ', 'factors', 'factor_spec'), ('Ⓟ', 'polynomial', 'polyval')]

__all__ = ['Function', 'fork', 'atop']
_primitives = {}
for _glyph, _monad, _dyad in _vocabulary:
    _inner = _Function.builtin(_glyph)
    _primitives[_glyph] = Function(_inner)
    for _valence, _name in enumerate((_monad, _dyad), 1):
        if _name is not None:
            globals()[_name] = Function(_inner, _valence)
            __all__.append(_name)

def _binary(glyph, x, y):
    f = _primitives[glyph]
    return fork(x, f, y) if isinstance(x, Function) or isinstance(y, Function) else f(x, y)

def _unary(glyph, y):
    f = _primitives[glyph]
    return atop(f, y) if isinstance(y, Function) else f(y)
