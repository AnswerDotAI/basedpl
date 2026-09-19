"Native APL arrays in an interruptible, thread-backed session."
import math, operator, sys, weakref
from dataclasses import dataclass
from fractions import Fraction
from ._core import __version__, symbols, _Array, _Function, _Session

__all__ = ['__version__', 'symbols', 'Array', 'Result', 'AplError', 'Session']

def _dtype(items):
    import numpy as np
    types = {type(o) for o in items}
    if types == {int} and all(-(1<<63) <= o < 1<<63 for o in items): return np.int64
    if types <= {float, complex}: return np.complex128 if complex in types else np.float64
    if types == {str}: return 'U1'
    return object

def _value(raw, as_array=False, session=None):
    def item(o):
        if isinstance(o, tuple): return Fraction(*o)
        if isinstance(o, _Function): return Function(o, session=session if o.needs_session else None)
        return _value(o, session=session) if isinstance(o, dict) else o
    if 'atom' in raw:
        result = item(raw['atom'])
        if not as_array: return result
        import numpy as np
        return np.array(result, dtype=_dtype([result]))
    shape, data = tuple(raw['shape']), [item(o) for o in raw['data']]
    items = raw['data'] or [raw['prototype']]
    nested = any(isinstance(o, dict) and 'shape' in o for o in items)
    if not as_array:
        if len(shape) == 1 and all(isinstance(o, str) for o in items): return ''.join(data)
    import numpy as np
    dtype = object if nested else _dtype(data or [item(raw['prototype'])])
    if dtype is not object: return np.array(data, dtype=dtype).reshape(shape)
    result = np.empty(len(data), dtype=object)
    for i,o in enumerate(data): result[i] = o
    return result.reshape(shape)

def _element(value, seen):
    np = sys.modules.get('numpy')
    if np is not None and isinstance(value, np.generic): value = value.item()
    if isinstance(value, bool): return int(value)
    if isinstance(value, Fraction): return value.numerator, value.denominator
    if isinstance(value, Function): return value._inner
    if type(value) in (int, float, complex): return value
    if isinstance(value, str) and len(value) == 1: return value
    if isinstance(value, (Array, str, list, tuple)) or np is not None and isinstance(value, np.ndarray): return _array(value, seen)
    raise TypeError(f'cannot convert {type(value).__name__} to APL')

def _rectangular(value, seen):
    if not isinstance(value, (list, tuple)): return (), [value]
    if not value: return (0,), []
    if id(value) in seen: raise ValueError('cyclic Python container')
    seen.add(id(value))
    try: parts = [_rectangular(o, seen) for o in value]
    finally: seen.remove(id(value))
    if any(shape != parts[0][0] for shape,_ in parts): return (len(value),), list(value)
    return (len(value),)+parts[0][0], [o for _,data in parts for o in data]

def _array(value, seen=None):
    if isinstance(value, Array): return value._inner
    if isinstance(value, _Array): return value
    if seen is None: seen = set()
    if id(value) in seen: raise ValueError('cyclic Python container')
    if len(seen) > 128: raise ValueError('array nesting exceeds 128 levels')
    np = sys.modules.get('numpy')
    prototype = 0.
    if isinstance(value, str): return _Array(dict(shape=[len(value)], data=list(value), prototype=' '))
    if np is not None and isinstance(value, np.ndarray):
        kind, size = value.dtype.kind, value.dtype.itemsize
        if kind not in 'biufcUO' or kind == 'f' and size > 8 or kind == 'c' and size > 16:
            raise TypeError('unsupported NumPy dtype')
        shape, data = value.shape, value.ravel().tolist()
        if kind in 'biu': prototype = 0
        elif kind == 'U': prototype = ' '
    else:
        shape, data = _rectangular(value, seen)
        if not shape: return _Array(dict(atom=_element(value, seen)))
    seen.add(id(value))
    try: return _Array(dict(shape=list(shape), data=[_element(o, seen) for o in data], prototype=prototype))
    finally: seen.remove(id(value))

class _Operators:
    __hash__ = None
    __array_priority__ = 1000
    def __add__(self, other): return _binary('+', self, other)
    def __radd__(self, other): return _binary('+', other, self)
    def __sub__(self, other): return _binary('-', self, other)
    def __rsub__(self, other): return _binary('-', other, self)
    def __mul__(self, other): return _binary('×', self, other)
    def __rmul__(self, other): return _binary('×', other, self)
    def __truediv__(self, other): return _binary('÷', self, other)
    def __rtruediv__(self, other): return _binary('÷', other, self)
    def __floordiv__(self, other): return _unary('⌊', _binary('÷', self, other))
    def __rfloordiv__(self, other): return _unary('⌊', _binary('÷', other, self))
    def __mod__(self, other): return _binary('|', other, self)
    def __rmod__(self, other): return _binary('|', self, other)
    def __pow__(self, other): return _binary('*', self, other)
    def __rpow__(self, other): return _binary('*', other, self)
    def __and__(self, other): return _binary('∧', self, other)
    def __rand__(self, other): return _binary('∧', other, self)
    def __or__(self, other): return _binary('∨', self, other)
    def __ror__(self, other): return _binary('∨', other, self)
    def __eq__(self, other): return _binary('=', self, other)
    def __ne__(self, other): return _binary('≠', self, other)
    def __lt__(self, other): return _binary('<', self, other)
    def __le__(self, other): return _binary('≤', self, other)
    def __gt__(self, other): return _binary('>', self, other)
    def __ge__(self, other): return _binary('≥', self, other)
    def __neg__(self): return _unary('-', self)
    def __pos__(self): return _unary('+', self)
    def __invert__(self): return _unary('~', self)

def _array_repr(raw):
    if 'atom' in raw: return f'Array({raw["atom"]!r})'
    def item(o):
        if isinstance(o, dict): return f'Array({_array_repr(o)})'
        if isinstance(o, tuple): return repr(Fraction(*o))
        text = repr(o)
        return text[:-1] if isinstance(o, float) and text.endswith('.0') else text
    shape, data = raw['shape'], iter(raw['data'])
    def cells(dims):
        if not dims: return item(next(data))
        return '[' + ', '.join(cells(dims[1:]) for _ in range(dims[0])) + ']'
    if math.prod(shape) == 0: return f'Array([], shape={tuple(shape)})'
    return cells(shape)

def _context(*values):
    session, pending, seen = None, list(values), set()
    while pending:
        value = pending.pop()
        if isinstance(value, (Array, Function)):
            owner = value._session
            if owner is not None:
                if session is not None and session is not owner: raise ValueError('cannot combine functions from different sessions')
                session = owner
        elif isinstance(value, (list, tuple)):
            if id(value) in seen: continue
            seen.add(id(value))
            pending.extend(value)
        else:
            np = sys.modules.get('numpy')
            if np is not None and isinstance(value, np.ndarray) and value.dtype.kind == 'O':
                if id(value) in seen: continue
                seen.add(id(value))
                pending.extend(value.flat)
    return session

class Array(_Operators):
    "An immutable native APL value. Conversion to Python or NumPy makes a copy."
    __slots__ = ('_inner', '_session')
    def __init__(self, value):
        self._inner = _array(value)
        self._session = _context(value) if self._inner.needs_session else None
    @property
    def shape(self): return tuple(self._inner.shape)
    @property
    def is_atom(self): return self._inner.is_atom
    @property
    def py(self): return _value(self._inner.parts(), session=self._session)
    @property
    def np(self): return _value(self._inner.parts(), as_array=True, session=self._session)
    @property
    def apl(self): return repr(self._inner)
    def _scalar(self): return _value(self._inner.scalar(), session=self._session)
    def __float__(self): return float(self._scalar())
    def __int__(self): return int(self._scalar())
    def __index__(self): return operator.index(self._scalar())
    def __bool__(self): return bool(self._scalar())
    def __len__(self):
        if not self.shape: raise TypeError('a scalar has no length')
        return self.shape[0]
    def __iter__(self): return (_wrap_array(a, self._session) for a in self._inner.cells())
    def __contains__(self, item): raise TypeError('use member for APL membership')
    def __getitem__(self, index):
        parts = index if isinstance(index, tuple) else (index,)
        def part(o):
            if not isinstance(o, slice): return _array(o)
            if o.start is not None or o.stop is not None or o.step is not None: raise TypeError('only a full : slice is supported; use APL index arrays')
            return None
        return _result(self._inner.select([part(o) for o in parts]), self._session).value
    def __matmul__(self, other):
        if isinstance(other, Function): raise TypeError('inner product requires two arrays or two functions')
        return plus.inner(times)(self, other)
    def __rmatmul__(self, other): return plus.inner(times)(other, self)
    def __array__(self, dtype=None, copy=None):
        if copy is False: raise ValueError('miniapl conversion requires a copy')
        result = self.np
        return result if dtype is None else result.astype(dtype, copy=False)
    def __repr__(self): return _array_repr(self._inner.parts())

@dataclass(frozen=True)
class Result:
    value:object
    output:list[str]

class AplError(RuntimeError):
    "An APL diagnostic, with retained source, UTF-8 byte spans, calls and captured output."
    def __init__(self, error, output):
        super().__init__(error['display'])
        self.kind, self.message = error['kind'], error['message']
        self.source_name, self.source, self.span = error['source']['name'], error['source']['text'], tuple(error['span'])
        self.output, self.calls = output, error['calls']

def _print(output):
    for text in output: print(text)

def _wrap_array(value, session=None):
    value = Array(value)
    if value._inner.needs_session: value._session = session
    return value

def _result(raw, session=None, display=False):
    if display: _print(raw['output'])
    if error := raw['error']: raise AplError(error, raw['output'])
    value = raw['value']
    if isinstance(value, _Function): value = Function(value, session=session if value.needs_session else None)
    elif value is not None: value = _wrap_array(value, session)
    return Result(value, raw['output'])

class Session:
    "Persistent APL worker thread. Calls return native values; eval captures explicit output."
    def __init__(self, timeout=None):
        if timeout is not None and (not math.isfinite(timeout) or timeout < 0): raise ValueError('timeout must be finite and nonnegative')
        self.timeout, self._worker = timeout, _Session()
        self._finalizer = weakref.finalize(self, self._worker.close)

    def _request(self, payload, display, echo=False):
        if not self._finalizer.alive: raise RuntimeError('session is closed')
        try: raw = self._worker.request(**payload, echo=echo, timeout=self.timeout)
        except KeyboardInterrupt as e:
            if display: _print(getattr(e, 'output', []))
            raise
        return _result(raw, self, display)

    def _eval(self, source, bindings, display, echo=False):
        def binding(v):
            owner = _context(v)
            if owner is not None and owner is not self: raise ValueError('function belongs to a different session')
            if not isinstance(v, Function): return _array(v)
            return v._inner
        payload = dict(bindings=[(k, binding(v)) for k,v in bindings.items()])
        if source is not None:
            if not isinstance(source, str): raise TypeError('APL source must be a string')
            payload['code'] = source
        return self._request(payload, display, echo)

    def __call__(self, source=None, /, **bindings):
        "Bind keyword arguments, then evaluate source. Print explicit output and return the value."
        return self._eval(source, bindings, True).value

    def eval(self, source=None, /, **bindings):
        "Bind keyword arguments, then return Result(value, output) without printing."
        return self._eval(source, bindings, False)

    def run(self, source, /, **bindings):
        "Capture APL session display, including implicit output, in Result(value, output)."
        return self._eval(source, bindings, display=False, echo=True)

    def __getitem__(self, source): return self(source)

    def __setitem__(self, name, value):
        if not isinstance(name, str): raise TypeError('binding name must be a string')
        self(**{name:value})

    def fn(self, source):
        "A composable late-bound Function: one argument is omega; two are alpha, omega."
        if not isinstance(source, str): raise TypeError('function expression must be a string')
        return _result(_Function.late_bound(source), self).value

    def interrupt(self):
        "Interrupt an evaluation from another thread."
        self._worker.interrupt()

    def close(self): self._finalizer()

    def __enter__(self):
        if not self._finalizer.alive: raise RuntimeError('session is closed')
        return self

    def __exit__(self, *args): self.close()

from .functions import *
from .functions import __all__ as _function_names, _binary, _unary
__all__ += _function_names
