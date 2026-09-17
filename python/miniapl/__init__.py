"A thin, copying Python boundary around the Rust APL interpreter."
from dataclasses import dataclass
from fractions import Fraction
from threading import current_thread
from ._core import __version__, _Session

__all__ = ['__version__', 'Array', 'Result', 'AplError', 'Session']

@dataclass(frozen=True)
class Array:
    "Copied APL shape, flat data, and prototype; no native ownership."
    shape:tuple
    data:tuple
    prototype:object

    @classmethod
    def from_numpy(cls, array):
        "Copy a numeric ndarray; integers must be exactly representable as APL floats."
        import numpy as np
        if type(array) is not np.ndarray: raise TypeError('from_numpy requires an ndarray')
        kind, size = array.dtype.kind, array.dtype.itemsize
        if kind not in 'biufc' or kind == 'f' and size > 8 or kind == 'c' and size > 16:
            raise TypeError('unsupported NumPy dtype')
        if not np.isfinite(array).all(): raise ValueError('APL numbers must be finite')
        source = array.ravel().tolist()
        data = tuple(complex(o) if kind == 'c' else float(o) for o in source)
        if kind in 'iu' and any(a != b for a,b in zip(source, data)):
            raise ValueError('integer cannot be represented exactly as an APL float')
        if kind == 'c': data = tuple(o.real if o.imag == 0 else o for o in data)
        return cls(array.shape, data, 0.)

    def to_python(self):
        "Return ordinary scalars/lists; this convenience loses empty-array prototype distinctions."
        items = iter(o.to_python() if isinstance(o, Array) else o for o in self.data)
        def build(shape): return [build(shape[1:]) for _ in range(shape[0])] if shape else next(items)
        return build(self.shape)

    def to_numpy(self):
        "Copy float/complex data into an ndarray, preserving shape but not the APL prototype."
        import numpy as np
        items = self.data or (self.prototype,)
        if any(type(o) not in (float, complex) for o in items):
            raise TypeError('to_numpy supports only float/complex arrays, not exact, character or nested values')
        dtype = np.complex128 if any(type(o) is complex for o in items) else np.float64
        return np.array(self.data, dtype=dtype).reshape(self.shape)


def _array(raw):
    def item(o):
        if isinstance(o, dict): return _array(o)
        return Fraction(*o) if isinstance(o, tuple) else o
    return Array(tuple(raw['shape']), tuple(item(o) for o in raw['data']), item(raw['prototype']))

def _raw(array):
    def item(o):
        if isinstance(o, Array): return _raw(o)
        return (o.numerator, o.denominator) if isinstance(o, Fraction) else o
    return dict(shape=array.shape, data=tuple(item(o) for o in array.data), prototype=item(array.prototype))

@dataclass(frozen=True)
class Result:
    value:Array | None
    output:list[str]

class AplError(RuntimeError):
    "A located APL error, including output produced before the failure."
    def __init__(self, error, output):
        super().__init__(error['display'])
        self.kind, self.message = error['kind'], error['message']
        self.source_name, self.source, self.span = error['source_name'], error['source'], error['span']
        self.output, self.calls = output, error['calls']

class Session:
    "Thread-affine session. Use, close, and destroy it on its creating thread."
    __slots__ = ('_native', '_thread')

    def __init__(self): self._native, self._thread = _Session(), current_thread()

    def _check_thread(self):
        if current_thread() is not self._thread: raise RuntimeError('session belongs to its creating thread')

    def eval(self, source, timeout=None):
        self._check_thread()
        if self._native is None: raise RuntimeError('session is closed')
        result = self._native.eval(source, timeout)
        if result['error'] is not None: raise AplError(result['error'], result['output'])
        return Result(None if result['value'] is None else _array(result['value']), result['output'])

    def set(self, name, value):
        "Bind a copied Array to an ordinary APL name."
        self._check_thread()
        if self._native is None: raise RuntimeError('session is closed')
        if not isinstance(value, Array): raise TypeError('set requires an Array')
        self._native.set(name, _raw(value))

    def close(self):
        self._check_thread()
        self._native = None

    def __enter__(self):
        self._check_thread()
        return self

    def __exit__(self, *args): self.close()
