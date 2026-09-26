"""bAsedPL is an APL-derived array language, borrowing ideas from J and BQN, with an emphasis on simple, consistent notation.

Modules:

- `basedpl.dyalog`: Run Dyalog APL through RIDE for reference checks.
- `basedpl.j`: Run the J language from Python and Jupyter through libj: sessions, magics and a Jupyter kernel."""

import math, operator, sys
from dataclasses import dataclass
from fractions import Fraction
from ._core import __version__, symbols, _Array, _Function, _Session

__all__ = ['__version__', 'symbols', 'Array', 'Result', 'AplError', 'apl', 'fn']

def _dtype(items):
    import numpy as np
    types = {type(o) for o in items}
    if types == {int} and all(-(1<<63) <= o < 1<<63 for o in items): return np.int64
    if types <= {float, complex}: return np.complex128 if complex in types else np.float64
    if types == {str}: return 'U1'
    return object


def _labels(keys):
    "Keys for Python, with its position for each unnamed entry."
    return [i if k is None else k for i,k in enumerate(keys)]


def _value(raw, as_array=False):
    def item(o):
        if isinstance(o, tuple): return Fraction(*o)
        if isinstance(o, _Function): return Function(o)
        return _value(o) if isinstance(o, dict) else o
    if 'atom' in raw:
        result = item(raw['atom'])
        if not as_array: return result
        import numpy as np
        return np.array(result, dtype=_dtype([result]))
    if not as_array and 'axis_keys' in raw and len(raw['shape']) > 1: return _dataframe(raw)
    shape, data = tuple(raw['shape']), [item(o) for o in raw['data']]
    items = raw['data'] or [raw['prototype']]
    nested = any(isinstance(o, dict) and 'shape' in o for o in items)
    if not as_array and 'axis_keys' in raw: return dict(zip(_labels(raw['axis_keys'][0]), data))
    if not as_array:
        if len(shape) == 1 and all(isinstance(o, str) for o in items): return ''.join(data)
    import numpy as np
    dtype = object if nested else _dtype(data or [item(raw['prototype'])])
    if dtype is not object: return np.array(data, dtype=dtype).reshape(shape)
    result = np.empty(len(data), dtype=object)
    for i,o in enumerate(data): result[i] = o
    return result.reshape(shape)

def _dataframe(raw):
    try: import pandas as pd
    except ImportError: raise ImportError('DataFrame conversion requires pandas: pip install pandas') from None
    data = _value(raw, as_array=True)
    shape = data.shape
    keys = raw.get('axis_keys', [None] * len(shape))
    names = raw.get('axis_names', [None] * len(shape))
    labels = [list(range(n)) if k is None else _labels(k) for n,k in zip(shape, keys)]
    if len(shape) < 2:
        index = pd.Index(labels[0], name=names[0]) if labels else [0]
        return pd.DataFrame(data.reshape(-1, 1), index=index, columns=[0])
    index = pd.Index(labels[0], name=names[0]) if len(shape) == 2 else pd.MultiIndex.from_product(labels[:-1], names=names[:-1])
    return pd.DataFrame(data.reshape(math.prod(shape[:-1]), shape[-1]), index=index, columns=pd.Index(labels[-1], name=names[-1]))

def _element(value, seen):
    np = sys.modules.get('numpy')
    if np is not None and isinstance(value, np.generic): value = value.item()
    if isinstance(value, bool): return int(value)
    if isinstance(value, Fraction): return value.numerator, value.denominator
    if isinstance(value, Function): return value._inner
    if type(value) in (int, float, complex): return value
    if isinstance(value, str) and len(value) == 1: return value
    if isinstance(value, (Array, str, list, tuple, dict)) or np is not None and isinstance(value, np.ndarray): return _array(value, seen)
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
    if isinstance(value, dict):
        keys = [None if type(k) is int and k == i else k for i,k in enumerate(value)]
        if not all(k is None or isinstance(k, str) for k in keys): raise TypeError('keyed arrays need string keys, or an integer key equal to its position')
        seen.add(id(value))
        try: return _Array(dict(shape=[len(value)], data=[_element(o, seen) for o in value.values()], prototype=prototype, axis_keys=[keys]))
        finally: seen.remove(id(value))
    if isinstance(value, str): return _Array(dict(shape=[len(value)], data=list(value), prototype=' '))
    if np is not None and isinstance(value, np.ndarray):
        kind, size = value.dtype.kind, value.dtype.itemsize
        if kind not in 'biufcUO' or kind == 'f' and size > 8 or kind == 'c' and size > 16: raise TypeError('unsupported NumPy dtype')
        if kind == 'u' and size == 8 and value.size and value.max() >= 1<<63: raise ValueError('uint64 values above the int64 range; convert with .astype(object) for exact integers')
        if kind in 'biu': return _Array.numeric(list(value.shape), np.ascontiguousarray(value, dtype=np.int64))
        if kind == 'f': return _Array.numeric(list(value.shape), np.ascontiguousarray(value, dtype=np.float64))
        shape, data = value.shape, value.ravel().tolist()
        if kind == 'U': prototype = ' '
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

def _text(raw):
    "The string in an unkeyed character vector, else `None`"
    if len(raw.get('shape', ())) != 1 or 'axis_keys' in raw or 'axis_names' in raw: return None
    if isinstance(raw['prototype'], str) and all(isinstance(c, str) for c in raw['data']): return ''.join(raw['data'])

def _array_repr(raw):
    if 'atom' in raw: return f'Array({raw["atom"]!r})'
    if (s := _text(raw)) is not None: return repr(s)
    def item(o):
        if isinstance(o, dict): return repr(t) if (t := _text(o)) is not None else f'Array({_array_repr(o)})'
        if isinstance(o, tuple): return repr(Fraction(*o))
        text = repr(o)
        return text[:-1] if isinstance(o, float) and text.endswith('.0') else text
    shape, data = raw['shape'], iter(raw['data'])
    def cells(dims):
        if not dims: return item(next(data))
        return '[' + ', '.join(cells(dims[1:]) for _ in range(dims[0])) + ']'
    if 'axis_names' in raw:
        attrs = ', '.join(f'{k}={raw[k]!r}' for k in ('axis_keys', 'axis_names') if k in raw)
        return f'Array({cells(shape)}, {attrs})'
    if 'axis_keys' in raw:
        if len(shape) == 1: return '{' + ', '.join(f'{k!r}: {item(o)}' for k,o in zip(_labels(raw['axis_keys'][0]), raw['data'])) + '}'
        return f'Array({cells(shape)}, axis_keys={raw["axis_keys"]!r})'
    if math.prod(shape) == 0: return f'Array([], shape={tuple(shape)})'
    return cells(shape)

class Array(_Operators):
    "An immutable native APL value. Conversion to Python or NumPy makes a copy."
    __slots__ = ('_inner',)
    def __init__(self, value, *, axis_keys=None, axis_names=None):
        self._inner = _array(value)
        if axis_keys is not None: self._inner = self._inner.with_axis_keys(axis_keys)
        if axis_names is not None: self._inner = self._inner.with_axis_names(axis_names)
    @property
    def shape(self): return tuple(self._inner.shape)
    @property
    def axis_keys(self): return tuple(None if k is None else tuple(k) for k in self._inner.axis_keys)
    @property
    def axis_names(self): return tuple(self._inner.axis_names)
    @property
    def is_atom(self): return self._inner.is_atom
    def _numpy(self):
        if (buffer := self._inner.buffer()) is None: return None
        import numpy as np
        return np.frombuffer(buffer[1], dtype=buffer[0]).reshape(self.shape)
    @property
    def py(self):
        keyed = any(k is not None for k in self._inner.axis_keys)
        if not (self.is_atom or keyed) and (result := self._numpy()) is not None: return result
        return _value(self._inner.parts())
    @property
    def np(self):
        result = self._numpy()
        return _value(self._inner.parts(), as_array=True) if result is None else result
    @property
    def df(self): return _dataframe(self._inner.parts())
    @property
    def apl(self): return repr(self._inner)
    def _scalar(self): return _value(self._inner.scalar())
    def __float__(self): return float(self._scalar())
    def __int__(self): return int(self._scalar())
    def __index__(self): return operator.index(self._scalar())
    def __bool__(self): return bool(self._scalar())
    def __len__(self):
        if not self.shape: raise TypeError('a scalar has no length')
        return self.shape[0]
    def __iter__(self): return (Array(a) for a in self._inner.cells())
    def __contains__(self, item): raise TypeError('use member for APL membership')
    def __getitem__(self, index):
        parts = index if isinstance(index, tuple) else (index,)
        def part(o):
            if not isinstance(o, slice): return _array(o)
            if o.start is not None or o.stop is not None or o.step is not None: raise TypeError('only a full : slice is supported; use APL index arrays')
            return None
        return _result(self._inner.select([part(o) for o in parts])).value
    def __matmul__(self, other):
        if isinstance(other, Function): raise TypeError('inner product requires two arrays or two functions')
        return _builtin('+').inner(_builtin('×'))(self, other)
    def __rmatmul__(self, other): return _builtin('+').inner(_builtin('×'))(other, self)
    def __array__(self, dtype=None, copy=None):
        if copy is False: raise ValueError('basedpl conversion requires a copy')
        result = self.np
        return result if dtype is None else result.astype(dtype, copy=False)
    def __repr__(self): return _array_repr(self._inner.parts())
    def _repr_mimebundle_(self, include=None, exclude=None):
        try: data = _builtin('•mime')(self).py
        except AplError: data = {'text/plain': self.apl}
        return {k:v for k,v in data.items() if (include is None or k in include) and (exclude is None or k not in exclude)}

@dataclass(frozen=True)
class Result:
    value:object
    events:list[dict]
    @property
    def output(self): return _output_text(self.events)

class AplError(RuntimeError):
    "An APL diagnostic, with retained source, UTF-8 byte spans, calls and captured output."
    def __init__(self, error, events):
        super().__init__(error['display'])
        self.kind, self.message = error['kind'], error['message']
        self.source_name, self.source, self.span = error['source']['name'], error['source']['text'], tuple(error['span'])
        self.events, self.calls = events, error['calls']
    @property
    def output(self): return _output_text(self.events)

def _output_text(events): return [e['data']['text/plain'] for e in events]

def _print(output):
    for text in output: print(text)

def _result(raw, display=False):
    if display: _print(_output_text(raw['output']))
    if error := raw['error']: raise AplError(error, raw['output'])
    value = raw['value']
    if isinstance(value, _Function): value = Function(value)
    elif value is not None: value = Array(value)
    return Result(value, raw['output'])

class _Workspace:
    "The APL workspace, `apl`. Calls return native values. `timeout` sets a per-evaluation deadline in seconds."
    def __init__(self): self.timeout, self._session = None, _Session()

    def _request(self, payload, display, echo=False):
        try: raw = self._session.request(**payload, echo=echo, timeout=self.timeout)
        except KeyboardInterrupt as e:
            if display: _print(getattr(e, 'output', []))
            raise
        return _result(raw, display)

    def _eval(self, source, bindings, display, echo=False):
        payload = dict(bindings=[(k, v._inner if isinstance(v, Function) else _array(v)) for k,v in bindings.items()])
        if source is not None:
            if not isinstance(source, str): raise TypeError('APL source must be a string')
            payload['code'] = source
        return self._request(payload, display, echo)

    def __call__(self, source=None, capture=None, /, **bindings):
        "Bind keyword arguments, then evaluate source. Print explicit output and return the value. With `capture` of 'explicit' or 'repl', return a `Result` without printing."
        if capture is None: return self._eval(source, bindings, True).value
        if capture not in ('explicit', 'repl'): raise ValueError(f"capture must be 'explicit' or 'repl', not {capture!r}")
        return self._eval(source, bindings, False, echo=capture == 'repl')

    def __getitem__(self, source): return self(source)

    def __getattr__(self, name):
        "Look up a builtin function by glyph, registered name/alias, or system name."
        return _builtin(name)

    def __dir__(self): return sorted(set(super().__dir__()) | _builtins.keys())

    def __setitem__(self, name, value):
        if not isinstance(name, str): raise TypeError('binding name must be a string')
        self(**{name:value})

    def fn(self, source):
        "A composable late-bound Function: one argument is omega; two are alpha, omega."
        if not isinstance(source, str): raise TypeError('function expression must be a string')
        return _result(_Function.late_bound(source)).value

    def names(self, prefix='', classes=(2,3,4)):
        "Sorted visible names, filtered by prefix and name class."
        return self._session.names(prefix, classes)

    def inspect(self, name):
        "Inspect a name or glyph without evaluating it; return None when absent."
        return self._session.inspect(name=name)

    def complete(self, prefix=''):
        "Complete user and system names."
        return self._session.names(prefix, complete=True)

    def interrupt(self):
        "Interrupt an evaluation from another thread."
        self._session.interrupt()

apl = _Workspace()
fn = apl.fn

from .functions import Function, fork, atop, __all__ as _function_names, _binary, _unary, _builtin, _builtins
from .printing import to_python
__all__ += ['to_python']
__all__ += _function_names

def __getattr__(name): return _builtin(name)
def __dir__(): return sorted(set(globals()) | _builtins.keys())

if 'IPython' in sys.modules:
    from .ipython import load_ipython_extension
    if (ip := sys.modules['IPython'].get_ipython()) is not None: load_ipython_extension(ip)
