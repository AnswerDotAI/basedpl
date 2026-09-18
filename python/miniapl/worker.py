"Persistent JSON-lines worker with cooperative interruption and a hard timeout fallback."
import json, math, queue, subprocess, sys, threading
from collections import deque
from decimal import Decimal

class Worker:
    "One request at a time. Interrupt from another thread; never retry a request automatically."
    def __init__(self, command=None):
        if command is None: command = (sys.executable, '-m', 'miniapl._cli', '--worker')
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, encoding='utf-8', bufsize=1, start_new_session=True)
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

    def eval(self, code, timeout=None, grace=1): return self.request(dict(code=code), timeout, grace)

    def interrupt(self):
        with self._write_lock:
            if self._active is None: return
            self.process.stdin.write(json.dumps(dict(interrupt=self._active))+'\n')
            self.process.stdin.flush()

    def request(self, payload, timeout=None, grace=1):
        "Return one result. A hard timeout closes the worker and raises TimeoutError (session lost)."
        if timeout is not None and (not math.isfinite(timeout) or timeout < 0): raise ValueError('timeout must be finite and nonnegative')
        if grace < 0 or not math.isfinite(grace): raise ValueError('grace must be finite and nonnegative')
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
                    e.output = []
                    self.interrupt()
                    try:
                        reply = self._replies.get(timeout=grace)
                        if isinstance(reply, dict): e.output = reply['result'].get('output', [])
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
