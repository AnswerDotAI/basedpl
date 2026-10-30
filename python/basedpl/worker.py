"Persistent JSON-lines worker with cooperative interruption and a hard timeout fallback."
import json, math, os, queue, subprocess, sysconfig, threading
from pathlib import Path
from collections import deque, namedtuple
from decimal import Decimal
from . import BplError, _output_text, _written
from .syntax import linear
from fastcore.meta import delegates

INSTALLED_BPL = Path(sysconfig.get_path('scripts'))/'bpl' # The `bpl` that `cargo develop` installs, a link through its installed build (`fastws.store`)
NATIVE = {'RUSTYMATH_NATIVE': '1'} # The environment that runs rustymath's own linear algebra in place of Accelerate, in a macOS build with rustymath's `native` feature

class Worker:
    "One request at a time. Interrupt from another thread; never retry a request automatically."
    def __init__(
        self,
        bpl=None, # The native `bpl` to run (`resolved`); None: the installed build
        native=False # Run rustymath's own linear algebra in place of Accelerate, through `RUSTYMATH_NATIVE=1`, in a macOS build with rustymath's `native` feature
    ):
        env = os.environ | NATIVE if native else None
        self.process = subprocess.Popen((resolved(bpl), '--worker'), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            encoding='utf-8', bufsize=1, start_new_session=True, env=env)
        self._replies, self._stderr = queue.Queue(), deque(maxlen=30)
        self._write_lock, self._request_lock = threading.Lock(), threading.Lock()
        self._id, self._active = 0, None
        self._readers = [threading.Thread(target=self._read, daemon=True), threading.Thread(target=self._read_errors, daemon=True)]
        for t in self._readers: t.start()

    def _read(self):
        try:
            for line in self.process.stdout: self._replies.put(json.loads(line, parse_int=lambda n: int(Decimal(n))))
        except Exception as e: self._replies.put(e)
        finally: self._replies.put(EOFError('worker exited; its session is lost'))

    def _read_errors(self):
        for line in self.process.stderr: self._stderr.append(line)

    @property
    def diagnostics(self): return ''.join(self._stderr)

    def eval(
        self,
        code, # BPL source
        timeout=None, # Seconds before the worker interrupts the evaluation
        grace=1 # Seconds to wait for the reply after `timeout`, or after a Ctrl-C, before closing the worker
    ):
        "Evaluate `code` in this worker's session and return the result, as `request` does."
        return self.request(dict(code=code), timeout, grace)

    def interrupt(self):
        "Ask the worker to stop the active request. It does nothing when no request is active."
        with self._write_lock:
            if self._active is None: return
            self.process.stdin.write(json.dumps(dict(interrupt=self._active))+'\n')
            self.process.stdin.flush()

    def request(
        self,
        payload, # Request fields, such as `code`, `reference`, `page`, `timing` or `costs`
        timeout=None, # Seconds before the worker interrupts the request
        grace=1 # Seconds to wait for the reply after `timeout`, or after a Ctrl-C, before closing the worker
    ):
        "Send one request to the worker and return its result. A Ctrl-C interrupts the request and re-raises with its partial `output` and `events`. If no reply arrives within `timeout` plus `grace`, the worker closes and its session is lost. The call then raises `TimeoutError`. A worker that exits, as a crash ends it, raises `EOFError` with its last lines of stderr."
        for name, seconds in [('timeout', timeout), ('grace', grace)]:
            if seconds is not None and (not math.isfinite(seconds) or seconds < 0): raise ValueError(f'{name} must be finite and nonnegative')
        with self._request_lock:
            if self.process.poll() is not None: raise RuntimeError('worker is closed; its session is lost')
            self._id += 1
            message = dict(payload, id=self._id)
            if timeout is not None: message['timeout_ms'] = math.ceil(timeout*1000)
            message = json.dumps(message, ensure_ascii=False, allow_nan=False)+'\n'
            try:
                try:
                    with self._write_lock:
                        self._active = self._id
                        self.process.stdin.write(message)
                        self.process.stdin.flush()
                    reply = self._replies.get(timeout=None if timeout is None else timeout+grace)
                except KeyboardInterrupt as e:
                    e.output, e.events = [], []
                    self.interrupt()
                    try:
                        reply = self._replies.get(timeout=grace)
                        if isinstance(reply, dict):
                            e.events = reply['result'].get('output', [])
                            e.output = _output_text(e.events)
                    except queue.Empty: self.close()
                    raise
                except queue.Empty:
                    self.close()
                    raise TimeoutError('worker did not stop before the deadline; its session is lost') from None
                if isinstance(reply, Exception):
                    self.close()
                    if isinstance(reply, EOFError) and self.diagnostics: raise EOFError(f'{reply}\n{self.diagnostics.rstrip()}')
                    raise reply
                if reply.get('id') != self._id:
                    self.close()
                    raise RuntimeError('worker response id mismatch')
                return reply['result']
            finally:
                with self._write_lock: self._active = None

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try: self.process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        for t in self._readers: t.join()
        for stream in (self.process.stdin, self.process.stdout, self.process.stderr): stream.close()

    def __enter__(self): return self
    def __exit__(self, *args): self.close()

def resolved(build=None):
    "The native `bpl` of `build`, resolved now, so that no later `cargo develop` changes it: a path to a `bpl` (`~` expands), a prefix of the id of a build that `cargo develop` kept (`fastws.store`), or None for the installed build."
    if build is None: return Path(os.path.realpath(INSTALLED_BPL))
    if (path := Path(build).expanduser()).exists(): return path
    from fastws.store import resolve
    return resolve(Path(__file__).resolve().parents[2], str(build))/'bin'/'bpl'

Output = namedtuple('Output', 'code output error')

def _commented(code, lines=()):
    "`code` with BPL comments that hold its linear form (`syntax.linear`), then each of `lines`. The comments go beside code of one line that fits in 80 characters with its linear form, and below other code."
    notes = [*('linear: ' + linear(code).replace('\n', '\n' + ' '*8)).split('\n'), *lines]
    head = f'{code}  ⍝ {notes[0]}'
    if '\n' in code or len(head) > 80: return '\n'.join([code, *(f'⍝ {n}' for n in notes)])
    return '\n'.join([head, *(f'{" "*(len(code)+2)}⍝ {n}' for n in notes[1:])])

class Outputs(list):
    "Each expression's printed text and error, as `Output` rows with fields `code`, `output` and `error`, where `error` is the `BplError`, `TimeoutError` or `EOFError` it raised, or `None`. Each row displays as BPL: its code, then comments that hold the code's linear form (`syntax.linear`) and its output or error, beside short code and below long code."
    def __repr__(self):
        def row(code, output, error): return _commented(code, '\n'.join(filter(None, [output, error and str(error)])).splitlines())
        return '\n'.join(row(*r) for r in self)

class Printed(str):
    "The text that `fresh` gives for one expression. It displays as a row of `Outputs` does, with the code's linear form before the text."
    def __new__(cls, text, code):
        self = super().__new__(cls, text)
        self.code = code
        return self
    def __repr__(self): return repr(Outputs([Output(self.code, str(self), None)]))

def _outcome(code, timeout, prefs, kwargs):
    "What `code` prints in a fresh worker after `prefs` runs, and the `BplError`, `TimeoutError` or `EOFError` it raised, or `None`. An error in `prefs` raises."
    with Worker(**kwargs) as w:
        if prefs and (r := w.eval(prefs, timeout))['error']: raise BplError(r['error'], r['output'])
        try: r = w.eval(code, timeout)
        except (TimeoutError, EOFError) as e: return '', e
    return _written(r['output']).removesuffix('\n'), r['error'] and BplError(r['error'], r['output'])

@delegates(Worker, but=['bpl'])
def fresh(
    code, # BPL source, or a list of expressions to run each in its own fresh worker
    timeout=60, # Seconds before each evaluation is interrupted, and a second more before an unresponsive worker is killed; None: no limit
    prefs='', # BPL that runs before `code` in each worker, such as `•prefs ["kinds":"static"]`
    build=None, # The build to run (`resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None for the installed build
    **kwargs
):
    "Return what `code` prints in a fresh worker process, as `Printed`. Its display shows the code's linear form (`syntax.linear`) before the text. A BPL error raises `BplError`, with the code and its linear form as a note. A worker that doesn't reply within `timeout` plus a second is killed, and `TimeoutError` raises. A worker that exits, as a crash ends it, raises `EOFError` with its last lines of stderr. A list of expressions gives `Outputs`, with each one's output or error, timeouts and exits included, and runs every one on the same build."
    kwargs |= dict(bpl=resolved(build))
    if not isinstance(code, str): return Outputs(Output(c, *_outcome(c, timeout, prefs, kwargs)) for c in code)
    output, error = _outcome(code, timeout, prefs, kwargs)
    if error:
        error.add_note(_commented(code))
        raise error
    return Printed(output, code)
