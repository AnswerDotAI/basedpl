import fcntl, os, signal, subprocess, sys
from contextlib import suppress
from fastws.procs import JOB_ENV, kill_group


def test_step_cleanup_after_leader_exit(tmp_path):
    child = '''import fcntl, signal, sys
f = open(sys.argv[1], 'a')
fcntl.flock(f, fcntl.LOCK_EX)
print('ready', flush=True)
signal.pause()
'''
    lock, pid = tmp_path/'held', tmp_path/'pid'
    command = f'''import os, pathlib, subprocess, sys
p = subprocess.Popen([sys.executable, '-u', '-c', {child!r}, {str(lock)!r}], stdout=subprocess.PIPE, text=True)
pathlib.Path({str(pid)!r}).write_text(str(os.getpid()) + ' ' + str(p.pid))
assert p.stdout.readline() == 'ready\\n'
'''
    runner = f'''from pathlib import Path
from basedpl.bpltests import _Step, _run
rows = _run([_Step('child', [{sys.executable!r}, '-c', {command!r}], Path({str(tmp_path)!r}), timeout=.2)], Path({str(tmp_path)!r}), 1, False)
print(rows[0][1])
'''
    p = subprocess.Popen([sys.executable, '-c', runner], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, start_new_session=True, env={k: v for k, v in os.environ.items() if k != JOB_ENV})
    try:
        out, err = p.communicate(timeout=5)
        assert p.returncode == 0, err
        assert out.strip() == 'ok' and pid.exists()
        with open(lock) as f: fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    finally:
        kill_group(p)
        if pid.exists():
            group, child_pid = map(int, pid.read_text().split())
            with suppress(ProcessLookupError): os.killpg(group, signal.SIGKILL)
            with suppress(ProcessLookupError): os.kill(child_pid, signal.SIGKILL)
