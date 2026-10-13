// The machine's timing lock, as DEV.md describes, for timings that run in Node: `tests/wasm-speed.mjs` and
// `basedpl.bpltests.speed_costs` with `wasm`. Node has no `flock`. A Python process takes the lock exclusively, waiting up to 5 minutes
// for builds and other timings, then holds it until its standard input closes. That happens when the Node process ends, even by a
// crash. After waiting 30 seconds it prints, once, the other processes that have the lock's file open. On macOS, where `lsof` doesn't
// show `flock` holds, that list also includes any other process waiting for the lock.
import { spawn } from 'node:child_process';

const LOCK = '/tmp/aai-ws-timing.lock';
const HOLD = `import fcntl, os, subprocess, sys, time
f = open(sys.argv[1], 'a')
start, waiting, reported = time.monotonic(), False, False
while True:
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
        break
    except BlockingIOError:
        if time.monotonic() - start > 300: sys.exit(f'a build or another timing has held {sys.argv[1]} for 5 minutes')
        if not waiting: print(f'waiting for a build or another timing to release {sys.argv[1]}', file=sys.stderr)
        if not reported and time.monotonic() - start > 30:
            users, pid = [], ''
            for line in subprocess.run(['lsof', '+c', '0', '-F', 'pc', sys.argv[1]], capture_output=True, text=True).stdout.splitlines():
                if line[:1] == 'p': pid = line[1:]
                elif line[:1] == 'c' and pid != str(os.getpid()): users.append(f'{pid} {line[1:]}')
            print(f'{sys.argv[1]} is still taken after 30 seconds. Processes with it open: {", ".join(users) or "none"}', file=sys.stderr)
            reported = True
        waiting = True
        time.sleep(0.25)
print('held', flush=True)
sys.stdin.read()`;

// Resolves to a function that releases the timing lock.
export function timingLock() {
    const holder = spawn('python3', ['-c', HOLD, LOCK], { stdio: ['pipe', 'pipe', 'inherit'] });
    return new Promise((resolve, reject) => {
        holder.stdout.once('data', () => resolve(() => holder.stdin.end()));
        holder.once('error', reject);
        holder.once('exit', code => reject(new Error(`the timing lock's holder exited with code ${code}`)));
    });
}
