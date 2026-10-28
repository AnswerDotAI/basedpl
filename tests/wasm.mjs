// Checks the browser module, built by `cargo wasm`, in Node. The depth checks run in a worker with 80% of a Chrome worker's stack,
// once with each of V8's compilers. The depth limit allows 120 levels of recursion that counts two a level, and 79 of
// recursion that counts three. Each shape must reach its depth, and recursion to 300 levels must stop with a LIMIT error before the
// stack runs out.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import { Session, pkg } from './wasm-pkg.mjs';

// The text of each output, and the error's kind, when `code` runs in a new session.
const run = code => {
    const outputs = [];
    const error = new Session().run(code, o => outputs.push(o.data['text/plain']));
    return { outputs, error: error?.kind };
};
// The MIME bundle of the last output when `code` runs in a new session.
const display = code => {
    let data;
    new Session().run(code, o => data = o.data);
    return data;
};

// Each shape, and the depth it must reach.
const shapes = {
    plain: ['{⍵=0?0;1+∇ ⍵-1}', 120],
    assigned: ['{⍵=0?0;x←∇ ⍵-1 ⋄ x+1}', 120],
    each: ['{⍵=0?0;1+↑f¨⍵-1}', 120],
    rank: ['{⍵=0?0;1+↑f⍤0⊢⍵-1}', 120],
    operand: ['{⍵=0?0;1+(f m) ⍵-1}', 120],
    outer: ['{⍵=0?0;1+↑(0 f⊗ ⍵-1)}', 79],
};

if (isMainThread && process.argv[2] !== 'depths') {
    test('runs code and streams its output', () => {
        assert.deepEqual(run('⎕←"hi" ⋄ +/⍳4'), { outputs: ['"hi"', '6'], error: undefined });
    });
    test('matches with JavaScript regular expressions and templates, at character positions', () => {
        const { outputs } = run('r←•r "(?<=é|\\s)(?<x>[bB])(\\d)?" ⋄ m←r.matches "éb1 B" ⋄ ⎕←m.position ⋄ ⎕←"<$<x>$2>" r.replace "éb1 B" ⋄ m.groups');
        assert.deepEqual(outputs, ['[1 4]ₓ', '"é<b1> <B>"', '("b" "1"⋄"B" "")']);
    });
    test('shows a picture as RGBA bytes with its width in the MIME type, and an animation as an animated PNG', () => {
        assert.deepEqual(display('•image 1 2⍴0 1')['image/x-rgba;width=2'], Uint8Array.of(0, 0, 0, 255, 255, 255, 255, 255));
        assert.ok(Buffer.from(display('•image 2 3 4 3⍴0.5')['image/png']).includes('acTL'));
    });
    test('converts values to JavaScript', () => {
        const s = new Session();
        assert.deepEqual(s.eval('[2.5 "ab" ["x":1 "y":$t]]'), { shape: [3], data: [2.5, 'ab', { x: 1, y: true }] });
        assert.deepEqual(s.eval('3 2⍴⍳6ₓ'), { shape: [3, 2], data: Uint8Array.of(0, 1, 2, 3, 4, 5) });
        assert.deepEqual(s.eval('1j2 3'), { shape: [2, 2], data: Float64Array.of(1, 2, 3, 0) });
        assert.deepEqual(s.eval('[1j2 3]ₛ'), { shape: [2, 2], data: Float32Array.of(1, 2, 3, 0) });
        assert.equal(s.eval('{}0'), undefined);
        assert.throws(() => s.eval('1÷"a"'), { kind: 'DOMAIN' });
    });
    test('binds JavaScript values and functions as BPL names, and makes functions with •js', () => {
        const s = new Session(), drawn = [];
        s.bind({ scale: 3, add: (a, b) => a + b, draw: d => { drawn.push(d); }, pts: { shape: [2, 2], data: Float64Array.of(1, 2, 3, 4) } });
        assert.equal(s.eval('scale add 2'), 5);
        assert.deepEqual(s.eval('+⌿pts'), { shape: [2], data: Float64Array.of(4, 6) });
        assert.equal(s.eval('draw 1 2'), undefined);
        assert.deepEqual(drawn, [{ shape: [2], data: Float64Array.of(1, 2) }]);
        assert.equal(s.eval('h←•js "Math.hypot" ⋄ 3 h 4'), 5);
        assert.deepEqual(s.eval('(•js "s => s.toUpperCase()")¨"ab" "cd"'), { shape: [2], data: ['AB', 'CD'] });
    });
    test('max and min beside NaN give NaN or the expected value, as the reference case max-min-nan checks natively', () => {
        // The case's code and its expected result are the two lines after its header in core.bpl.
        const lines = readFileSync(new URL('reference/core.bpl', import.meta.url), 'utf8').split('\n');
        const at = lines.findIndex(l => l.startsWith('⍝ max-min-nan '));
        assert.notEqual(at, -1, 'core.bpl holds the case max-min-nan');
        const s = new Session();
        assert.deepEqual(s.eval(lines[at + 1]), s.eval(lines[at + 2]));
    });
    test('a canvas output carries JSON and bytes, which canvas.js draws', () => {
        const data = display('d←•canvas "(ctx, data, options) => ctx.fillRect(options.x, 0, data.data[1], data.shape[0])" ⋄ ["x":5] d 3 2⍴⍳6');
        const calls = [], show = (0, eval)(readFileSync(new URL('canvas.js', pkg), 'utf8'));
        show({ getContext: () => ({ fillRect: (...a) => calls.push(a) }) }, data['application/x-bpl-canvas+json'], data['application/x-bpl-canvas-data']);
        assert.deepEqual(calls, [[5, 0, 1, 3]]);
    });
    test('each recursion shape reaches its depth, and deeper recursion is a LIMIT error, under both V8 compilers', () => {
        // V8 compiles each function first with Liftoff, then recompiles hot functions with TurboFan, whose frames can be larger. V8's
        // flags apply to a whole process, so each compiler runs the checks in a process of its own.
        for (const flags of [[], ['--no-liftoff']]) {
            const out = execFileSync(process.execPath, [...flags, fileURLToPath(import.meta.url), 'depths'], { encoding: 'utf8' });
            for (const [name, [n, reached, deep]] of Object.entries(JSON.parse(out))) {
                assert.equal(reached, n, `${name} ${flags}`);
                assert.equal(deep, 'LIMIT', `${name} ${flags}`);
            }
        }
    });
} else if (isMainThread) {
    // `depths KB` gives the worker KB of stack. `depths KB NAME N` runs shape NAME alone to depth N, as `basedpl.bpltests.wasm_stack` does.
    // Node keeps 192 KB of a worker's stack in reserve. `chromeMb` is that reserve plus the stack a Chrome worker gives, about 470 KB. Without
    // KB, the worker gets the reserve and 80% of Chrome's part. The other 20% is a margin for frames that grow between V8 versions and builds.
    const [kb, name, n] = process.argv.slice(3), reserveMb = 0.1875, chromeMb = 0.65;
    new Worker(new URL(import.meta.url), { workerData: name && { [name]: [shapes[name][0], Number(n)] },
                                           resourceLimits: { stackSizeMb: kb ? kb / 1024 : reserveMb + 0.8 * (chromeMb - reserveMb) } })
        .on('message', results => console.log(JSON.stringify(results)));
} else {
    const depth = (shape, n) => { const { outputs, error } = run(`m←{⍶ ⍵} ⋄ f←${shape} ⋄ f ${n}`); return error ?? Number(outputs.at(-1)); };
    // Each shape's depth, the depth its recursion reached, and what recursion to 300 levels gave.
    const results = {};
    for (const [name, [shape, n]] of Object.entries(workerData ?? shapes)) {
        const result = results[name] = [n];
        try { result.push(depth(shape, n)); result.push(depth(shape, 300)); }
        // A stack overflow leaves the module's own stack pointer where it was, so no check can follow it.
        catch (e) { if (!(e instanceof RangeError)) throw e; result.push('OVERFLOW'); break; }
    }
    parentPort.postMessage(results);
}
