// Times the cases in `speed.txt` in the browser module, built by `cargo wasm`, against their wasm budgets, as `tests/speed.rs` times
// them against their native budgets. Both call the same timing code, `basedpl::timing::costs`, through the module's `costs`. Run it
// as `node --no-liftoff --test tests/wasm-speed.mjs`. V8 first compiles wasm with Liftoff, which doesn't optimise, and recompiles a
// function with TurboFan only once it runs often. The test calls each kernel only a few times. Without the flag, it would time
// Liftoff's code. To print every case's cost, run `BASEDPL_COSTS=1 node --no-liftoff tests/wasm-speed.mjs`.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { costs } from './wasm-pkg.mjs';
import { timingLock } from './timing-lock.mjs';

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
