// Times the cases in `speed/*.bpl` in the browser module, built by `cargo wasm`, against their wasm budgets, as `tests/speed.rs` times
// them against their native budgets. Both call the same timing code, `basedpl::timing::costs`, through the module's `costs`. Run it
// as `node --no-liftoff --test tests/wasm-speed.mjs`. V8 first compiles wasm with Liftoff, which doesn't optimise, and recompiles a
// function with TurboFan only once it runs often. The test calls each kernel only a few times. Without the flag, it would time
// Liftoff's code. A case over its budget is timed again after the rest, after each pause in `RETRY_PAUSES`, and fails only if it's
// over every time, as in `tests/speed.rs`. With `BASEDPL_COSTS` set, it also prints every case's cost. The `Test` workflow's
// `wasm` job sets it. Node can't lock a file. `basedpl.bpltests.wasm_test` holds the machine's timing lock around this test.
// Run directly, as CI runs it, the test takes no lock.
// The wasm budgets serve CI's x86 runner, as `speed/README.md` describes. On any other host, the test prints every cost, then
// each case still over its budget after the retries, marked `over`, and fails on none, as `bpl --costs` does.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';
import { costs } from './wasm-pkg.mjs';

// The pauses before each retry, in seconds, as in `rustymath::timing::RETRY_PAUSES`.
const RETRY_PAUSES = [0.1, 1];
const ci = process.platform === 'linux' && process.arch === 'x64';

assert.ok(process.execArgv.includes('--no-liftoff'), 'run with node --no-liftoff');
test('each BPL speed case takes no longer than its wasm budget on CI\'s runner', async () => {
    const directory = new URL('speed/', import.meta.url);
    const speed = readdirSync(directory).filter(name => name.endsWith('.bpl')).sort()
        .map(name => ({name, text: readFileSync(new URL(name, directory), 'utf8')}));
    const line = c => c.line;
    const over = ({ budgets: [, , budget], cost }) => cost > Number(budget);
    const all = costs(speed);
    if (process.env.BASEDPL_COSTS || !ci) {
        for (const copy of new Set(all.map(c => c.copy).filter(Boolean))) console.log(copy);
        for (const c of all) console.log(line(c));
    }
    let failures = all.filter(over);
    for (const pause of RETRY_PAUSES) {
        if (!failures.length) break;
        await new Promise(resolve => setTimeout(resolve, pause * 1000));
        for (const c of failures) console.log(`${line(c)}: over, timed again after ${pause} s`);
        const retried = costs(speed, failures.map(c => c.label)).filter(c => failures.some(f => f.file === c.file && f.section === c.section && f.code === c.code));
        for (const c of retried) console.log(line(c));
        failures = retried.filter(over);
    }
    if (ci) assert.deepEqual(failures.map(line), []);
    else for (const c of failures) console.log(`${line(c)}: over`);
});
