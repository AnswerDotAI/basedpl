"Persistent JSON-lines worker with cooperative interruption and a hard timeout fallback."
import json, math, os, queue, subprocess, sysconfig, threading
from pathlib import Path
from collections import deque, namedtuple
from decimal import Decimal
from . import BplError, _output_text, _written
from fastcore.meta import delegates

INSTALLED_BPL = Path(sysconfig.get_path('scripts'))/'bpl' # The `bpl` that `cargo develop` installs and replaces
NATIVE = {'RUSTYMATH_NATIVE': '1'} # The environment that runs rustymath's own linear algebra in place of Accelerate, in a macOS build with rustymath's `native` feature

class Worker:
    "One request at a time. Interrupt from another thread; never retry a request automatically."
    def __init__(
        self,
        bpl=None, # A native `bpl` to run, such as a copy of `INSTALLED_BPL` saved before a change; None: `INSTALLED_BPL`
        native=False # Run rustymath's own linear algebra in place of Accelerate, through `RUSTYMATH_NATIVE=1`, in a macOS build with rustymath's `native` feature
    ):
        env = os.environ | NATIVE if native else None
        self.process = subprocess.Popen((bpl or INSTALLED_BPL, '--worker'), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
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
        "Send one request to the worker and return its result. A Ctrl-C interrupts the request and re-raises with its partial `output` and `events`. If no reply arrives within `timeout` plus `grace`, the worker closes and its session is lost. The call then raises `TimeoutError`."
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

Output = namedtuple('Output', 'code output error')

class Outputs(list):
    "Each expression's printed text and error, as `Output` rows with fields `code`, `output` and `error`, where `error` is the `BplError` or `TimeoutError` it raised, or `None`. Each row displays as BPL, with its output or error after `⍝`."
    def __repr__(self):
        def row(code, output, error):
            lines = '\n'.join(filter(None, [output, error and str(error)])).split('\n')
            return code + '  ⍝ ' + f'\n{" "*(len(code)+2)}⍝ '.join(lines)
        return '\n'.join(row(*r) for r in self)

def _outcome(code, timeout, prefs, kwargs):
    "What `code` prints in a fresh worker after `prefs` runs, and the `BplError` or `TimeoutError` it raised, or `None`. An error in `prefs` raises."
    with Worker(**kwargs) as w:
        if prefs and (r := w.eval(prefs, timeout))['error']: raise BplError(r['error'], r['output'])
        try: r = w.eval(code, timeout)
        except TimeoutError as e: return '', e
    return _written(r['output']).removesuffix('\n'), r['error'] and BplError(r['error'], r['output'])

@delegates(Worker)
def fresh(
    code, # BPL source, or a list of expressions to run each in its own fresh worker
    timeout=60, # Seconds before each evaluation is interrupted, and a second more before an unresponsive worker is killed; None: no limit
    prefs='', # BPL that runs before `code` in each worker, such as `•prefs ["kinds":"static"]`
    **kwargs
):
    "Return what `code` prints in a fresh worker process. A BPL error raises `BplError`. A worker that doesn't reply within `timeout` plus a second is killed, and `TimeoutError` raises. A list of expressions gives `Outputs`, with each one's output or error, timeouts included."
    if not isinstance(code, str): return Outputs(Output(c, *_outcome(c, timeout, prefs, kwargs)) for c in code)
    output, error = _outcome(code, timeout, prefs, kwargs)
    if error: raise error
    return output
