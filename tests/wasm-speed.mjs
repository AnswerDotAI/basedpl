// Times the cases in `speed.txt` in the browser module, built by `cargo wasm`, against their wasm budgets, as `tests/speed.rs` times
// them against their native budgets. Both call the same timing code, `basedpl::timing::costs`, through the module's `costs`. Run it
// as `node --no-liftoff --test tests/wasm-speed.mjs`. V8 first compiles wasm with Liftoff, which doesn't optimise, and recompiles a
// function with TurboFan only once it runs often. The test calls each kernel only a few times. Without the flag, it would time
// Liftoff's code. To print every case's cost, run `BASEDPL_COSTS=1 node --no-liftoff tests/wasm-speed.mjs`.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { costs } from './wasm-pkg.mjs';

// The file whose lock keeps timings and builds apart on this machine, as DEV.md describes. Node has no `flock`. A Python process takes
// the lock exclusively, waiting up to 5 minutes for builds and other timings, then holds it until its standard input closes. That
// happens when this process ends, even by a crash.
const LOCK = '/tmp/aai-ws-timing.lock';
const HOLD = `import fcntl, sys, time
f = open(sys.argv[1], 'a')
deadline, waiting = time.monotonic() + 300, False
while True:
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
        break
    except BlockingIOError:
        if time.monotonic() > deadline: sys.exit(f'a build or another timing has held {sys.argv[1]} for 5 minutes')
        if not waiting: print(f'waiting for a build or another timing to release {sys.argv[1]}', file=sys.stderr)
        waiting = True
        time.sleep(0.25)
print('held', flush=True)
sys.stdin.read()`;

// Resolves to a function that releases the timing lock.
function timingLock() {
    const holder = spawn('python3', ['-c', HOLD, LOCK], { stdio: ['pipe', 'pipe', 'inherit'] });
    return new Promise((resolve, reject) => {
        holder.stdout.once('data', () => resolve(() => holder.stdin.end()));
        holder.once('error', reject);
        holder.once('exit', code => reject(new Error(`the timing lock's holder exited with code ${code}`)));
    });
}

assert.ok(process.execArgv.includes('--no-liftoff'), 'run with node --no-liftoff');
test('each case in speed.txt takes no longer than its wasm budget', async () => {
    const release = await timingLock();
    const failures = [];
    try {
        for (const { section, code, budgets: [, budget], cost } of costs(readFileSync(new URL('speed.txt', import.meta.url), 'utf8'))) {
            const result = `${section} ${code}: ${cost.toFixed(2)}, budget ${budget}`;
            if (process.env.BASEDPL_COSTS) console.log(result);
            if (cost > Number(budget)) failures.push(result);
        }
    } finally { release(); }
    assert.deepEqual(failures, []);
});
