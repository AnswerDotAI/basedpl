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

    def to_python(self):
        "Return ordinary scalars/lists; this convenience loses empty-array prototype distinctions."
        items = iter(o.to_python() if isinstance(o, Array) else o for o in self.data)
        def build(shape): return [build(shape[1:]) for _ in range(shape[0])] if shape else next(items)
        return build(self.shape)


def _array(raw):
    def item(o):
        if isinstance(o, dict): return _array(o)
        return Fraction(*o) if isinstance(o, tuple) else o
    return Array(tuple(raw['shape']), tuple(item(o) for o in raw['data']), item(raw['prototype']))

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

    def eval(self, source):
        self._check_thread()
        if self._native is None: raise RuntimeError('session is closed')
        result = self._native.eval(source)
        if result['error'] is not None: raise AplError(result['error'], result['output'])
        return Result(None if result['value'] is None else _array(result['value']), result['output'])

    def close(self):
        self._check_thread()
        self._native = None

    def __enter__(self):
        self._check_thread()
        return self

    def __exit__(self, *args): self.close()
