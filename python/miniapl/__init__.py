"Python values and NumPy arrays in an interruptible APL session."
import math, weakref
from dataclasses import dataclass
from decimal import Decimal
from fractions import Fraction
import numpy as np
from ._core import __version__, symbols
from .worker import Worker

__all__ = ['__version__', 'symbols', 'Result', 'AplError', 'Session']

def _dtype(items):
    types = {type(o) for o in items}
    if types == {int} and all(-(1<<63) <= o < 1<<63 for o in items): return np.int64
    if types <= {float, complex}: return np.complex128 if complex in types else np.float64
    if types == {str}: return 'U1'
    return object

def _value(raw):
    def item(o):
        if not isinstance(o, dict): return o
        if 'rational' in o: return Fraction(*(int(Decimal(n)) for n in o['rational']))
        if 'complex' in o: return complex(*o['complex'])
        return _value(o)
    shape, data = tuple(raw['shape']), [item(o) for o in raw['data']]
    items = raw['data'] or [raw['prototype']]
    nested = any(isinstance(o, dict) and 'shape' in o for o in items)
    if not shape and not nested: return data[0]
    dtype = object if nested else _dtype(data or [item(raw['prototype'])])
    if len(shape) == 1 and dtype == 'U1': return ''.join(data)
    if dtype is not object: return np.array(data, dtype=dtype).reshape(shape)
    result = np.empty(len(data), dtype=object)
    for i,o in enumerate(data): result[i] = o
    return result.reshape(shape)

def _element(value):
    if isinstance(value, np.generic): value = value.item()
    if isinstance(value, bool): return int(value)
    if isinstance(value, Fraction) or type(value) is int and not -(1<<63) <= value < 1<<63:
        value = Fraction(value)
        return dict(rational=[str(Decimal(value.numerator)), str(Decimal(value.denominator))])
    if type(value) is int: return value
    if type(value) in (float, complex):
        if not np.isfinite(value): raise ValueError('APL numbers must be finite')
        return dict(complex=[value.real, value.imag]) if type(value) is complex else value
    if isinstance(value, str) and len(value) == 1: return value
    if isinstance(value, (str, list, tuple, np.ndarray)): return _array(value)
    raise TypeError(f'cannot convert {type(value).__name__} to APL')

def _rectangular(value):
    if not isinstance(value, (list, tuple)): return (), [value]
    if not value: return (0,), []
    parts = [_rectangular(o) for o in value]
    if any(shape != parts[0][0] for shape,_ in parts): return (len(value),), list(value)
    return (len(value),)+parts[0][0], [o for _,data in parts for o in data]

def _array(value):
    prototype = 0.
    if isinstance(value, str): return dict(shape=[len(value)], data=list(value), prototype=' ')
    if isinstance(value, np.ndarray):
        kind, size = value.dtype.kind, value.dtype.itemsize
        if kind not in 'biufcUO' or kind == 'f' and size > 8 or kind == 'c' and size > 16:
            raise TypeError('unsupported NumPy dtype')
        shape, data = value.shape, value.ravel().tolist()
        if kind in 'biu': prototype = 0
        elif kind == 'U': prototype = ' '
    else: shape, data = _rectangular(value)
    return dict(shape=list(shape), data=[_element(o) for o in data], prototype=prototype)

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

class Session:
    "Persistent APL worker. Calls return Python values; eval captures explicit output."
    def __init__(self, timeout=None):
        if timeout is not None and (not math.isfinite(timeout) or timeout < 0): raise ValueError('timeout must be finite and nonnegative')
        self.timeout, self._worker = timeout, Worker()
        self._finalizer = weakref.finalize(self, self._worker.close)

    def _request(self, payload, display, echo=False):
        if not self._finalizer.alive: raise RuntimeError('session is closed')
        try: raw = self._worker.request(dict(payload, echo=echo), timeout=self.timeout)
        except KeyboardInterrupt as e:
            if display: _print(getattr(e, 'output', []))
            raise
        if display: _print(raw['output'])
        if error := raw['error']:
            if error['kind'] == 'REQUEST ERROR': raise ValueError(error['message'])
            raise AplError(error, raw['output'])
        return Result(None if raw['value'] is None else _value(raw['value']), raw['output'])

    def _eval(self, source, bindings, display, echo=False):
        payload = dict(bindings={k:_array(v) for k,v in bindings.items()})
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
        "A late-bound Python callable: one argument is omega; two are alpha, omega."
        if not isinstance(source, str): raise TypeError('function expression must be a string')
        def f(*args):
            if len(args) not in (1, 2): raise TypeError('APL functions take one or two arguments')
            return self._request(dict(call=source, args=[_array(o) for o in args]), True).value
        return f

    def interrupt(self):
        "Interrupt an evaluation from another thread."
        self._worker.interrupt()

    def close(self): self._finalizer()

    def __enter__(self):
        if not self._finalizer.alive: raise RuntimeError('session is closed')
        return self

    def __exit__(self, *args): self.close()
