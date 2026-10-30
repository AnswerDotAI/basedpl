// Times the cases in `speed/*.bpl` in the browser module, built by `cargo wasm`, against their wasm budgets, as `tests/speed.rs` times
// them against their native budgets. Both call the same timing code, `basedpl::timing::costs`, through the module's `costs`. Run it
// as `node --no-liftoff --test tests/wasm-speed.mjs`. V8 first compiles wasm with Liftoff, which doesn't optimise, and recompiles a
// function with TurboFan only once it runs often. The test calls each kernel only a few times. Without the flag, it would time
// Liftoff's code. A case over its budget is timed again after the rest, after each pause in `RETRY_PAUSES` and a `warm`, and fails only if it's
// over every time, as in `tests/speed.rs`. With `BASEDPL_COSTS` set, it also prints every case's cost. The `Test` workflow's
// `wasm` job sets it. Node can't lock a file. `basedpl.bpltests.wasm_test` holds the machine's timing lock around this test.
// Run directly, as CI runs it, the test takes no lock.
// The test fails on a wasm budget only on the CPU that the budgets were set for, CI's runner, as `checks_budgets` decides. On any
// other CPU, it prints every cost, then each case still over its budget after the retries, marked `over`, and fails on none.
// Each case whose gap's hold has outlived the gap is printed too, marked `stale`. The rows that `costs` gives carry each case's
// wasm budget with any hold, and whether its cost is over it or its hold is stale, as `rustymath::timing::Budget` decides.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';
import { checks_budgets, costs, retry_pauses, warm } from './wasm-pkg.mjs';

// The pauses before each retry, in seconds, from `rustymath::timing::RETRY_PAUSES`.
const RETRY_PAUSES = retry_pauses();
const checks = checks_budgets();

assert.ok(process.execArgv.includes('--no-liftoff'), 'run with node --no-liftoff');
test('each BPL speed case takes no longer than its wasm budget on CI\'s runner', async () => {
    const directory = new URL('speed/', import.meta.url);
    const speed = readdirSync(directory).filter(name => name.endsWith('.bpl')).sort()
        .map(name => ({name, text: readFileSync(new URL(name, directory), 'utf8')}));
    const line = c => c.line;
    const over = c => c.over;
    const all = costs(speed);
    if (process.env.BASEDPL_COSTS || !checks) {
        for (const copy of new Set(all.map(c => c.copy).filter(Boolean))) console.log(copy);
        for (const c of all) console.log(line(c));
    }
    let failures = all.filter(over);
    for (const pause of RETRY_PAUSES) {
        if (!failures.length) break;
        await new Promise(resolve => setTimeout(resolve, pause * 1000));
        warm();
        for (const c of failures) console.log(`${line(c)}: over, timed again after ${pause} s`);
        const retried = costs(speed, failures.map(c => c.label)).filter(c => failures.some(f => f.file === c.file && f.section === c.section && f.code === c.code));
        for (const c of retried) console.log(line(c));
        failures = retried.filter(over);
    }
    for (const c of all.filter(c => c.stale)) console.log(`${line(c)}: stale`);
    if (checks) assert.deepEqual(failures.map(line), []);
    else for (const c of failures) console.log(`${line(c)}: over`);
});
