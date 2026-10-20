"""BasedPL, the Based-array Programming Language, is an APL-derived array language, borrowing ideas from J and BQN, with an emphasis on simple, consistent notation.

Modules:

- `basedpl.dyalog`: Run Dyalog APL through RIDE for reference checks.
- `basedpl.editors`: Write editor highlighters, glyph-entry templates, regional macOS keyboards, and glyph-page kinds, keys and ranks.
- `basedpl.keyboards`: Generate regional BPL keyboards from macOS layouts."""

import math, operator, sys
from dataclasses import dataclass
from fractions import Fraction
from ._core import __version__, symbols, _Array, _Function, _Operator, _Session

__all__ = ['__version__', 'symbols', 'Array', 'Result', 'BplError', 'bpl', 'fn']
__pyskill_sigs__ = False

def _dtype(items):
    import numpy as np
    types = {type(o) for o in items}
    if types == {bool}: return np.bool_
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
        if isinstance(o, _Operator): return Operator(o)
        return _value(o) if isinstance(o, (dict, _Array)) else o
    if isinstance(raw, _Array):
        if not as_array and (text := raw.text) is not None: return text
        keys = raw.keys
        if not as_array and any(k is not None for k in keys): return _dataframe(raw) if len(raw.shape) > 1 else dict(zip(_labels(keys[0]), raw.tolist()))
        import numpy as np
        return np.array(raw)
    if 'atom' in raw:
        result = item(raw['atom'])
        if not as_array: return result
        import numpy as np
        return np.array(result, dtype=_dtype([result]))
    if not as_array and 'axis_keys' in raw and len(raw['shape']) > 1: return _dataframe(raw)
    shape, data = tuple(raw['shape']), [item(o) for o in raw['data']]
    items = raw['data'] or [raw['prototype']]
    nested = any(isinstance(o, _Array) or isinstance(o, dict) and 'shape' in o for o in items)
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
    keys, names = (raw.keys, raw.names) if isinstance(raw, _Array) else (raw.get('axis_keys', [None] * len(shape)), raw.get('axis_names', [None] * len(shape)))
    labels = [list(range(n)) if k is None else _labels(k) for n,k in zip(shape, keys)]
    if len(shape) < 2:
        index = pd.Index(labels[0], name=names[0]) if labels else [0]
        return pd.DataFrame(data.reshape(-1, 1), index=index, columns=[0])
    index = pd.Index(labels[0], name=names[0]) if len(shape) == 2 else pd.MultiIndex.from_product(labels[:-1], names=names[:-1])
    return pd.DataFrame(data.reshape(math.prod(shape[:-1]), shape[-1]), index=index, columns=pd.Index(labels[-1], name=names[-1]))

def _element(value, seen):
    np = sys.modules.get('numpy')
    if np is not None and isinstance(value, np.generic):
        if value.dtype.kind in 'biufc': return _array(value.__array__(), seen).native_atom()
        value = value.item()
    if isinstance(value, bool): return value
    if isinstance(value, Fraction): return value.numerator, value.denominator
    if isinstance(value, (Function, Operator)): return value._inner
    if type(value) in (int, float, complex): return value
    if isinstance(value, str) and len(value) == 1: return value
    if isinstance(value, (Array, str, list, tuple, range, dict)) or hasattr(value, '__array__'): return _array(value, seen)
    if (read := _Array.numeric(value)) is not None: return read
    raise TypeError(f'cannot convert {type(value).__name__} to BPL')

def _rectangular(value, seen):
    if not isinstance(value, (list, tuple, range)): return (), [value]
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
    np = sys.modules.get('numpy')
    numpy = np is not None and isinstance(value, np.ndarray)
    if numpy:
        kind, size = value.dtype.kind, value.dtype.itemsize
        if kind not in 'biufcUO' or kind == 'f' and size > 8 or kind == 'c' and size > 16: raise TypeError('unsupported NumPy dtype')
        # The object path preserves unsigned values outside int64 as exact integers.
        if kind == 'u' and size == 8 and value.size and value.max() >= 1<<63: value, kind = value.astype(object), 'O'
    prototype = 0.
    if isinstance(value, dict):
        keys = [None if type(k) is int and k == i else k for i,k in enumerate(value)]
        if not all(k is None or isinstance(k, str) for k in keys): raise TypeError('keyed arrays need string keys, or an integer key equal to its position')
        seen.add(id(value))
        try: return bpl._session.array(dict(shape=[len(value)], data=[_element(o, seen) for o in value.values()], prototype=prototype, axis_keys=[keys]))
        finally: seen.remove(id(value))
    if isinstance(value, str): return bpl._session.array(dict(shape=[len(value)], data=list(value), prototype=' '))
    # Any object with a buffer, such as a NumPy or rustymath array or a `memoryview`, is read through it.
    if not (numpy and kind in 'UO') and (read := _Array.numeric(value)) is not None: return read
    if not numpy and hasattr(value, '__array__'):
        seen.add(id(value))
        try: return _array(value.__array__(), seen)
        finally: seen.remove(id(value))
    if numpy:
        shape, data = value.shape, value.ravel().tolist()
        if kind == 'U': prototype = ' '
    else:
        shape, data = _rectangular(value, seen)
        if not shape: return bpl._session.array(dict(atom=_element(value, seen)))
    seen.add(id(value))
    try: return bpl._session.array(dict(shape=list(shape), data=[_element(o, seen) for o in data], prototype=prototype))
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
    def __and__(self, other): return _bitwise('∧', self, other)
    def __rand__(self, other): return _bitwise('∧', other, self)
    def __or__(self, other): return _bitwise('∨', self, other)
    def __ror__(self, other): return _bitwise('∨', other, self)
    def __eq__(self, other): return _binary('=', self, other)
    def __ne__(self, other): return _binary('≠', self, other)
    def __lt__(self, other): return _binary('<', self, other)
    def __le__(self, other): return _binary('≤', self, other)
    def __gt__(self, other): return _binary('>', self, other)
    def __ge__(self, other): return _binary('≥', self, other)
    def __neg__(self): return _unary('-', self)
    def __pos__(self): return _unary('+', self)
    def __invert__(self): return _bitwise('~', self)
    def __abs__(self): return _unary('|', self)

def _text(raw):
    "The string in an unkeyed character vector, else `None`"
    if isinstance(raw, _Array): return raw.text
    if len(raw.get('shape', ())) != 1 or 'axis_keys' in raw or 'axis_names' in raw: return None
    if isinstance(raw['prototype'], str) and all(isinstance(c, str) for c in raw['data']): return ''.join(raw['data'])

def _raw(raw):
    "`parts` as a dict: a compact array's `_Array` gives its shape, keys, names and items from `tolist`, with no NumPy"
    if not isinstance(raw, _Array): return raw
    shape, data = list(raw.shape), raw.tolist()
    if not shape: data = [data]
    for _ in shape[1:]: data = [o for row in data for o in row]
    result = dict(shape=shape, data=data)
    if any(k is not None for k in raw.keys): result['axis_keys'] = list(raw.keys)
    if any(n is not None for n in raw.names): result['axis_names'] = list(raw.names)
    return result

def _array_repr(raw):
    if (s := _text(raw)) is not None: return repr(s)
    raw = _raw(raw)
    if 'atom' in raw: return f'Array({raw["atom"]!r})'
    def item(o):
        if isinstance(o, (dict, _Array)): return repr(t) if (t := _text(o)) is not None else f'Array({_array_repr(o)})'
        if isinstance(o, tuple): return repr(Fraction(*o))
        text = repr(o)
        return text[:-1] if isinstance(o, float) and text.endswith('.0') else text
    shape, data = raw['shape'], iter(raw['data'])
    def cells(dims):
        if not dims: return item(next(data))
        return '[' + ', '.join(cells(dims[1:]) for _ in range(dims[0])) + ']'
    if 'axis_names' in raw:
        attrs = ', '.join(f'{k.removeprefix("axis_")}={raw[k]!r}' for k in ('axis_keys', 'axis_names') if k in raw)
        return f'Array({cells(shape)}, {attrs})'
    if 'axis_keys' in raw:
        if len(shape) == 1: return '{' + ', '.join(f'{k!r}: {item(o)}' for k,o in zip(_labels(raw['axis_keys'][0]), raw['data'])) + '}'
        return f'Array({cells(shape)}, keys={raw["axis_keys"]!r})'
    if math.prod(shape) == 0: return f'Array([], shape={tuple(shape)})'
    return cells(shape)

class Array(_Operators):
    "A native BPL value with shape, keys and names."
    __slots__ = ('_inner',)
    def __init__(self, value, *, keys=None, names=None):
        self._inner = value._inner.copy() if isinstance(value, Array) else _array(value)
        if keys is not None: self.keys = keys
        if names is not None: self.names = names
    @property
    def shape(self): return tuple(self._inner.shape)
    @property
    def keys(self): return tuple(None if k is None else tuple(k) for k in self._inner.keys)
    @keys.setter
    def keys(self, value): self._inner = self._inner.with_keys(value)
    @property
    def names(self): return tuple(self._inner.names)
    @names.setter
    def names(self, value): self._inner = self._inner.with_names(value)
    @property
    def is_atom(self): return self._inner.is_atom
    def _numpy(self, copy=True):
        if not self._inner.exports: return None
        import numpy as np
        return np.array(self._inner, copy=copy)
    @property
    def py(self):
        keyed = any(k is not None for k in self._inner.keys)
        if not (self.is_atom or keyed) and self._inner.text is None and (result := self._numpy()) is not None: return result
        return _value(self._inner.parts())
    @property
    def np(self):
        result = self._numpy()
        return _value(self._inner.parts(), as_array=True) if result is None else result
    def own(self):
        "The NumPy array that `.np` gives, with the items moved where nothing else holds them, and copied otherwise. The array holds nothing afterwards, and using it raises `ValueError`."
        result = None if self._inner.exports else self.np
        if (owned := self._inner.own()) is None: return result
        import numpy as np
        return np.asarray(owned)
    @property
    def df(self): return _dataframe(self._inner.parts())
    @property
    def bpl(self): return repr(self._inner)
    def _atom(self): return _value(self._inner.atom())
    def __float__(self): return float(self._atom())
    def __int__(self): return int(self._atom())
    def __index__(self): return operator.index(self._atom())
    def __bool__(self):
        if math.prod(self.shape) != 1: raise ValueError('an array with other than one item has ambiguous truth')
        return bool(self._atom())
    def __len__(self):
        if not self.shape: raise TypeError('a unit has no length')
        return self.shape[0]
    def __iter__(self): return (Array(a) for a in self._inner.cells())
    def __contains__(self, item):
        # A one-character string is an atom. A longer one is one item where the major cells are items, as a list of strings holds them, and otherwise a character vector, which matches a row of characters.
        if isinstance(item, str) and len(item) == 1: query = bpl._session.array(dict(atom=item))
        elif isinstance(item, str) and len(self.shape) == 1: query = bpl._session.array(dict(shape=[], data=[_array(item)], prototype=' '))
        else: query = _array(item)
        return self._inner.contains(query, bpl._session)
    def __getitem__(self, index):
        "Select by positions, keys, slices or per-axis masks. `None` inserts an axis and `...` keeps omitted axes."
        return _result(self._inner.select(index, _array)).value
    def __setitem__(self, index, value): self._inner = self._inner.assign(index, _array(value), _array)
    def __rustymath_array__(self): return self._inner.__rustymath_array__()
    def __matmul__(self, other): return _matmul(self, other)
    def __rmatmul__(self, other): return _matmul(other, self)
    def __array__(self, dtype=None, copy=None):
        result = self._numpy(copy=copy is not False)
        if result is None:
            if copy is False: raise ValueError('basedpl conversion requires a copy')
            result = self.np
        if copy is False and dtype is not None and result.dtype != dtype: raise ValueError('basedpl conversion requires a copy')
        return result if dtype is None else result.astype(dtype, copy=False)
    def __repr__(self): return _array_repr(self._inner.parts())
    def _repr_mimebundle_(self, include=None, exclude=None):
        try: data = _builtin('•mime')(self).py
        except BplError: data = {'text/plain': self.bpl}
        return {k: v if isinstance(v, str) else bytes(v) for k,v in data.items() if (include is None or k in include) and (exclude is None or k not in exclude)}

@dataclass(frozen=True)
class Result:
    value:object
    events:list[dict]
    @property
    def output(self): return _output_text(self.events)

class BplError(RuntimeError):
    "A BPL diagnostic, with retained source, character spans, calls and captured output."
    def __init__(self, error, events):
        super().__init__(error['display'])
        self.kind, self.message = error['kind'], error['message']
        self.source_name, self.source, self.span = error['source']['name'], error['source']['text'], tuple(error['span'])
        self.events, self.calls = events, error['calls']
    @property
    def output(self): return _output_text(self.events)

def _output_text(events): return [e['data']['text/plain'] for e in events]

def _written(events):
    "The text of `events` as a terminal shows it: a line ending follows each event except a `text` event, which BPL writes as it is."
    return ''.join(e['data']['text/plain'] + ('' if e['kind']=='text' else '\n') for e in events)

def _show(event):
    "Print one output event as a terminal shows it."
    print(_written([event]), end='')

def _result(raw):
    if error := raw['error']: raise BplError(error, raw['output'])
    value = raw['value']
    if isinstance(value, _Function): value = Function(value)
    elif isinstance(value, _Operator): value = Operator(value)
    elif value is not None: value = Array(value)
    return Result(value, raw['output'])

class _Workspace:
    "The BPL workspace, `bpl`. Calls return native values. `timeout` sets a per-evaluation deadline in seconds."
    __pyskill_sigs__ = False
    def __init__(self): self.timeout, self._session = None, _Session()

    def _eval(self, source, bindings, show=None, echo=False):
        "Bind `bindings`, then run `source`, calling `show` with each output event as BPL produces it."
        if source is not None and not isinstance(source, str): raise TypeError('BPL source must be a string')
        bindings = [(k, v._inner if isinstance(v, Function) else _array(v)) for k,v in bindings.items()]
        return _result(self._session.eval(code=source, bindings=bindings, timeout=self.timeout, echo=echo, show=show))

    def _call(self, function, args, show=None, weak=()):
        "Call the native `function` with the native `args`, calling `show` with each output event as BPL produces it. `weak` marks the arguments that came from Python scalars."
        return _result(self._session.call(function, args, weak=list(weak), timeout=self.timeout, show=show))

    def __call__(self, source=None, capture=None, /, **bindings):
        "Bind keyword arguments, then evaluate source. Print explicit output and return the value. With `capture` of 'explicit' or 'repl', return a `Result` without printing."
        if capture is None: return self._eval(source, bindings, _show).value
        if capture not in ('explicit', 'repl'): raise ValueError(f"capture must be 'explicit' or 'repl', not {capture!r}")
        return self._eval(source, bindings, echo=capture == 'repl')

    def __getitem__(self, source): return self(source)

    def __getattr__(self, name):
        "Look up a builtin function by glyph, registered name/alias or system name, then a workspace name."
        try: return _builtin(name)
        except AttributeError:
            if name.startswith('_') or name not in self.names(name): raise
        return self(name)

    def __dir__(self): return sorted(set(super().__dir__()) | _builtins.keys() | set(self.names()))

    def __setitem__(self, name, value):
        if not isinstance(name, str): raise TypeError('binding name must be a string')
        self(**{name:value})

    def fn(self, source):
        "A composable late-bound Function: one argument is omega; two are alpha, omega."
        if not isinstance(source, str): raise TypeError('function expression must be a string')
        return _result(self._session.late_bound(source)).value

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

bpl = _Workspace()
fn = bpl.fn

from .functions import Function, Operator, fork, atop, __all__ as _function_names, _binary, _unary, _bitwise, _matmul, _builtin, _builtins, _glyph
from .printing import to_python
from .syntax import parse
__all__ += ['to_python', 'parse']
__all__ += _function_names

def __getattr__(name): return _builtin(name)
def __dir__(): return sorted(set(globals()) | _builtins.keys())

if 'IPython' in sys.modules:
    from .ipython import load_ipython_extension
    if (ip := sys.modules['IPython'].get_ipython()) is not None: load_ipython_extension(ip)
