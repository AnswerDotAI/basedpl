"Persistent JSON-lines worker with cooperative interruption and a hard timeout fallback."
import json, math, os, queue, subprocess, sysconfig, threading, time
from pathlib import Path
from typing import Annotated
from collections import deque, namedtuple
from decimal import Decimal
from . import BplError, _output_text, _written
from .syntax import linear
from fastcore.meta import delegates
from contextlib import contextmanager

INSTALLED_BPL = Path(sysconfig.get_path('scripts'))/'bpl' # The `bpl` that `cargo develop` installs, a link through its installed build (`fastws.store`)
Compiles = Annotated[str, {'choices': ['always', 'never']}] # The values that `environment`'s `compile` takes, as choices for a shell argument
def environment(
    native=False, # Run rustymath's own linear algebra in place of Accelerate, through `RUSTYMATH_NATIVE=1`, in a macOS build with rustymath's `native` feature
    compile:Compiles=None, # How block stages compile, through `BASEDPL_COMPILE` (`settings::Compiles`): `always`, every stage on its first run; `never`, none; None, by the compile policy
):
    "This process's environment, with these settings, for a `bpl` process or speed test that it starts. Every tool that starts one takes them through `@delegates(environment)`."
    return os.environ | ({'RUSTYMATH_NATIVE': '1'} if native else {}) | ({'BASEDPL_COMPILE': compile} if compile else {})

LIMIT = 30 # Seconds of work that a check or timing call gets unless its `timeout` says otherwise; builds and waits for the timing lock don't count

class Limit:
    "The seconds of work left to one call that `seconds` bounds, with no bound for `None`. Each piece of the call's work spends the seconds it runs: a worker's request spends its `ran`, and `running` spends a block's seconds. Waits for builds and for the timing lock spend nothing. A tool's `timeout` takes seconds or a `Limit`, so a caller can share one limit across the tools it calls (`of`)."
    def __init__(self, seconds=LIMIT): self.seconds, self.spent, self.since = seconds, 0, None

    @classmethod
    def of(cls, timeout): return timeout if isinstance(timeout, Limit) else cls(timeout)

    def passed(self): return TimeoutError(f'the call reached its {self.seconds:g} s limit; check or time less at once, or pass a larger `timeout`')

    def left(self):
        "The seconds left, or `None` for no bound. Raises `TimeoutError` once none are left."
        if self.seconds is None: return None
        left = self.seconds - self.spent - (0 if self.since is None else time.monotonic() - self.since)
        if left <= 0: raise self.passed()
        return left

    def spend(self, seconds): self.spent += seconds

    def reached(self, error):
        "Raise `TimeoutError` where `error`, a request's error, is the timeout that the request's deadline gave."
        if error and error.get('kind') == 'TIMEOUT': raise self.passed()

    @contextmanager
    def running(self):
        "Spend the seconds that the block runs, such as processes that run while the call holds the timing lock."
        self.since = time.monotonic()
        try: yield
        finally: self.spent, self.since = self.spent + time.monotonic() - self.since, None

class Worker:
    "One request at a time. Interrupt from another thread; never retry a request automatically. Inherit the caller's process group; this transport stops its worker, while the containing job owns descendant cleanup."
    @delegates(environment)
    def __init__(
        self,
        bpl=None, # The native `bpl` to run (`resolved`); None or `installed`: the installed build
        stderr_limit=30, # Maximum diagnostic lines to retain; None keeps all stderr
        cwd=None, # Working directory of the child; None inherits the caller's directory
        **kwargs
    ):
        self.cwd = None if cwd is None else Path(cwd).expanduser().resolve()
        self.process = subprocess.Popen((resolved(bpl), '--worker'), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            encoding='utf-8', bufsize=1, env=environment(**kwargs), cwd=self.cwd)
        self._replies, self._stderr = queue.Queue(), deque(maxlen=stderr_limit)
        self._write_lock, self._request_lock = threading.Lock(), threading.Lock()
        self._id, self._active, self._heard, self._started, self._waiting, self.ran = 0, None, time.monotonic(), time.monotonic(), False, None
        self._readers = [threading.Thread(target=self._read, daemon=True), threading.Thread(target=self._read_errors, daemon=True)]
        for t in self._readers: t.start()

    def _read(self):
        try:
            with self.process.stdout as stream:
                for line in stream: self._replies.put(json.loads(line, parse_int=lambda n: int(Decimal(n))))
        except Exception as e: self._replies.put(e)
        finally: self._replies.put(EOFError('worker exited; its session is lost'))

    def _read_errors(self):
        with self.process.stderr as stream:
            for line in stream:
                if line == 'started\n': self._started, self._waiting = time.monotonic(), False
                else:
                    self._stderr.append(line)
                    if line.startswith('waiting for ') and ' to release ' in line: self._waiting = True
                self._heard = time.monotonic()

    def _reply(self, within, stall):
        "The next reply, which must arrive within `within` seconds of the request, or of the worker's `started` note where a timing request writes one once it holds the timing lock (`host::timing_lock`), and within `stall` seconds of the worker's last line of stderr or of the request, or else `TimeoutError`. While the worker waits for the lock, as its `waiting for … to release` note says, neither limit runs: the lock gives up after 5 minutes itself. `None` sets no limit. `ran` then holds the seconds from that start to the reply."
        self._started = self._heard = time.monotonic()
        self._waiting = False
        while True:
            if self._waiting: left = .25
            else:
                ends = [t for t in (None if within is None else self._started+within, None if stall is None else self._heard+stall) if t is not None]
                left = min(ends) - time.monotonic() if ends else None
                if left is not None and left <= 0:
                    late = within is not None and time.monotonic() >= self._started+within
                    raise TimeoutError(f'worker {"did not stop before the deadline" if late else f"wrote nothing for {stall} seconds"}; its session is lost')
            try: reply = self._replies.get(timeout=left)
            except queue.Empty: continue
            self.ran = time.monotonic() - self._started
            return reply

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
        timeout=None, # Seconds before the worker stops the request, counted from its `started` note where a timing request writes one once it holds the timing lock
        grace=1, # Seconds to wait for the reply after `timeout`, or after a Ctrl-C, before closing the worker
        stall=None # Seconds the worker may write no line to stderr, as a reference or page check notes each case or example it starts, before it counts as stuck
    ):
        "Send one request to the worker and return its result. A Ctrl-C interrupts the request and re-raises with its partial `output` and `events`. If no reply arrives within `timeout` plus `grace`, or within `stall` seconds of the worker's last line of stderr, the worker closes and its session is lost, since a deadlocked worker never takes the interrupt. The call then raises `TimeoutError`. A worker that exits, as a crash ends it, raises `EOFError` with its last lines of stderr."
        for name, seconds in [('timeout', timeout), ('grace', grace), ('stall', stall)]:
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
                    reply = self._reply(None if timeout is None else timeout+grace, stall)
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
                except TimeoutError:
                    self.close()
                    raise
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
        "Stop and reap the worker without waiting on pipes inherited by its descendants. Reader threads close their streams when their writers end."
        if self.process.poll() is None: self.process.kill(); self.process.wait()
        self.process.stdin.close()

    def __enter__(self): return self
    def __exit__(self, *args): self.close()

def _installed(build):
    "Whether `build` names the installed build: None, or `installed`, which a shell can write where it can't write None."
    return build is None or build == 'installed'

def resolved(build=None):
    "The native `bpl` of `build`, resolved now, so that no later `cargo develop` changes it: a path to a `bpl` (`~` expands), a prefix of the id of a build that `cargo develop` kept (`fastws.store`), or None or `installed` for the installed build."
    if _installed(build): return Path(os.path.realpath(INSTALLED_BPL))
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

def _outcome(code, limit, prefs, kwargs):
    "What `code` prints in a fresh worker after `prefs` runs, and the `BplError`, `TimeoutError` or `EOFError` it raised, or `None`. Both evaluations spend `limit`. An error in `prefs` raises."
    with Worker(**kwargs) as w:
        try:
            if prefs:
                if (r := w.eval(prefs, limit.left()))['error']: raise BplError(r['error'], r['output'])
                limit.spend(w.ran)
            r = w.eval(code, limit.left())
            limit.spend(w.ran)
        except (TimeoutError, EOFError) as e: return '', e
    return _written(r['output']).removesuffix('\n'), r['error'] and BplError(r['error'], r['output'])

@delegates(Worker, but=['bpl'])
def fresh(
    code, # BPL source, or a list of expressions to run each in its own fresh worker
    timeout=LIMIT, # Seconds of work that the call gets, or a `Limit` to share (`Limit.of`): each evaluation is interrupted once they're spent, and its worker killed a second later; None: no limit
    prefs='', # BPL that runs before `code` in each worker, such as `•prefs ["kinds":"static"]`
    build=None, # The build to run (`resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build
    **kwargs
):
    "Return what `code` prints in a fresh worker process, as `Printed`. Its display shows the code's linear form (`syntax.linear`) before the text. A BPL error raises `BplError`, with the code and its linear form as a note. A worker that doesn't reply within `timeout` plus a second is killed, and `TimeoutError` raises. A list's expressions share `timeout`. A worker that exits, as a crash ends it, raises `EOFError` with its last lines of stderr. A list of expressions gives `Outputs`, with each one's output or error, timeouts and exits included, and runs every one on the same build."
    kwargs |= dict(bpl=resolved(build))
    limit = Limit.of(timeout)
    if not isinstance(code, str): return Outputs(Output(c, *_outcome(c, limit, prefs, kwargs)) for c in code)
    output, error = _outcome(code, limit, prefs, kwargs)
    if error:
        error.add_note(_commented(code))
        raise error
    return Printed(output, code)
