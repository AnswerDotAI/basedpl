// Times the cases in `speed.txt` in the browser module, built by `cargo wasm`, against their wasm budgets, as `tests/speed.rs` times
// them against their native budgets. Run it as `node --no-liftoff --test tests/wasm-speed.mjs`. V8 first compiles wasm with Liftoff,
// which doesn't optimise, and recompiles a function with TurboFan only once it runs often. The test calls each kernel only a few
// times. Without the flag, it would time Liftoff's code. To print every case's cost, run
// `BASEDPL_COSTS=1 node --no-liftoff tests/wasm-speed.mjs`.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { Session } from './wasm-pkg.mjs';

assert.ok(process.execArgv.includes('--no-liftoff'), 'run with node --no-liftoff');
test('each case in speed.txt takes no longer than its wasm budget', () => {
    const sections = {};
    for (const line of readFileSync(new URL('speed.txt', import.meta.url), 'utf8').split('\n')) {
        const header = line.match(/^\[(\w+)\](?: (.*))?$/);
        if (header) sections[header[1]] = { prefs: header[2] ?? '', lines: [] };
        else if (line && !line.startsWith('#')) Object.values(sections).at(-1).lines.push(line);
    }
    const { setup, ...cases } = sections, sessions = {};
    const ok = (session, code) => assert.equal(session.run(code, () => {}), null, code);
    for (const [name, { prefs }] of Object.entries(cases)) ok(sessions[name] = new Session(), `${prefs}\n${setup.lines.join('\n')}`);
    // The fastest of five calls of `f`, after one call to warm up, in milliseconds.
    const fastest = f => {
        f();
        let best = Infinity;
        for (let i = 0; i < 5; i++) { const start = performance.now(); f(); best = Math.min(best, performance.now() - start); }
        return best;
    };
    const time = (name, code) => fastest(() => ok(sessions[name], `_←${code}`));
    // Holding the last copy stops V8 from dropping the allocation.
    const data = Float64Array.from({ length: sessions.copies.eval('n') }, (_, i) => i), kept = [];
    const copy = () => fastest(() => kept[0] = new Float64Array(data));
    const failures = [];
    for (const [name, { lines }] of Object.entries(cases)) {
        for (const line of lines) {
            const [code, , budget] = line.split('\t');
            const cost = name === 'widths' ? time(name, code) / time('exact', code) : time(name, code) / copy();
            const result = `${name} ${code}: ${cost.toFixed(2)}, budget ${budget}`;
            if (process.env.BASEDPL_COSTS) console.log(result);
            if (cost > Number(budget)) failures.push(result);
        }
    }
    assert.deepEqual(failures, []);
});
