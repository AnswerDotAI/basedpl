// Times the cases in `speed.txt` in the browser module, built by `cargo wasm`, against their wasm budgets, as `tests/speed.rs` times
// them against their native budgets. Both call the same timing code, `basedpl::timing::costs`, through the module's `costs`. Run it
// as `node --no-liftoff --test tests/wasm-speed.mjs`. V8 first compiles wasm with Liftoff, which doesn't optimise, and recompiles a
// function with TurboFan only once it runs often. The test calls each kernel only a few times. Without the flag, it would time
// Liftoff's code. A case over its budget is timed again after the rest, after each pause in `RETRY_PAUSES`, and fails only if it's
// over every time, as in `tests/speed.rs`. To print every case's cost, run `BASEDPL_COSTS=1 node --no-liftoff tests/wasm-speed.mjs`.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { costs } from './wasm-pkg.mjs';
import { timingLock } from './timing-lock.mjs';

// The pauses before each retry, in seconds, as in `basedpl::timing::RETRY_PAUSES`.
const RETRY_PAUSES = [0.1, 1];

assert.ok(process.execArgv.includes('--no-liftoff'), 'run with node --no-liftoff');
test('each case in speed.txt takes no longer than its wasm budget', async () => {
    const release = await timingLock();
    const speed = readFileSync(new URL('speed.txt', import.meta.url), 'utf8');
    const line = ({ section, code, budgets: [, budget], cost }) => `${section} ${code}: ${cost.toFixed(2)}, budget ${budget}`;
    const over = ({ budgets: [, budget], cost }) => cost > Number(budget);
    let failures = [];
    try {
        const all = costs(speed);
        if (process.env.BASEDPL_COSTS) for (const c of all) console.log(line(c));
        failures = all.filter(over);
        for (const pause of RETRY_PAUSES) {
            if (!failures.length) break;
            await new Promise(resolve => setTimeout(resolve, pause * 1000));
            for (const c of failures) console.log(`${line(c)}: over, timed again after ${pause} s`);
            const retried = costs(speed, failures.map(c => c.code)).filter(c => failures.some(f => f.section === c.section && f.code === c.code));
            for (const c of retried) console.log(line(c));
            failures = retried.filter(over);
        }
    } finally { release(); }
    assert.deepEqual(failures.map(line), []);
});
