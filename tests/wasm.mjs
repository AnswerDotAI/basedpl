// Checks the browser module, built by `cargo wasm`, in Node. The depth checks run in a worker with a stack about the size of a Chrome
// worker's. The call limit allows 188 levels of recursion that makes two calls a level, and 125 of recursion that makes three. Each
// shape must reach its depth, and recursion to 300 levels must stop with a LIMIT error before the stack runs out.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { Worker, isMainThread, parentPort } from 'node:worker_threads';
import init, { Session, configure } from '../wasm/pkg/basedpl_wasm.js';

await init({ module_or_path: readFileSync(new URL('../wasm/pkg/basedpl_wasm_bg.wasm', import.meta.url)) });
configure('http://localhost/', message => { throw new Error(message); });
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
    plain: ['{⍵=0?0;1+∇ ⍵-1}', 188],
    assigned: ['{⍵=0?0;x←∇ ⍵-1 ⋄ x+1}', 188],
    each: ['{⍵=0?0;1+↑f¨⍵-1}', 188],
    rank: ['{⍵=0?0;1+↑f⍤0⊢⍵-1}', 188],
    operand: ['{⍵=0?0;1+(f m) ⍵-1}', 188],
    outer: ['{⍵=0?0;1+↑(0 f⊗ ⍵-1)}', 125],
};

if (isMainThread) {
    test('runs code and streams its output', () => {
        assert.deepEqual(run('⎕←"hi" ⋄ +/⍳4'), { outputs: ['hi', '6'], error: undefined });
    });
    test('matches with JavaScript regular expressions and templates, at character positions', () => {
        const { outputs } = run('r←•r "(?<=é|\\s)(?<x>[bB])(\\d)?" ⋄ m←r.matches "éb1 B" ⋄ ⎕←m.position ⋄ ⎕←"<$<x>$2>" r.replace "éb1 B" ⋄ m.groups');
        assert.deepEqual(outputs, ['[1 4]ₓ', 'é<b1> <B>', '("b" "1"⋄"B" "")']);
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
    test('a canvas output carries JSON and bytes, which canvas.js draws', () => {
        const data = display('d←•canvas "(ctx, data, options) => ctx.fillRect(options.x, 0, data.data[1], data.shape[0])" ⋄ ["x":5] d 3 2⍴⍳6');
        const calls = [], show = (0, eval)(readFileSync(new URL('../wasm/pkg/canvas.js', import.meta.url), 'utf8'));
        show({ getContext: () => ({ fillRect: (...a) => calls.push(a) }) }, data['application/x-bpl-canvas+json'], data['application/x-bpl-canvas-data']);
        assert.deepEqual(calls, [[5, 0, 1, 3]]);
    });
    test('each recursion shape reaches its depth, and deeper recursion is a LIMIT error', async () => {
        // Node keeps 192 KB of a worker's stack in reserve. The rest, about 470 KB, is what a Chrome worker gives.
        const worker = new Worker(new URL(import.meta.url), { resourceLimits: { stackSizeMb: 0.65 } });
        const results = await new Promise((done, failed) => { worker.on('message', done); worker.on('error', failed); });
        for (const [name, [reached, deep]] of Object.entries(results)) {
            assert.equal(reached, shapes[name][1], name);
            assert.equal(deep, 'LIMIT', name);
        }
    });
} else {
    const depth = (shape, n) => { const { outputs, error } = run(`m←{⍶ ⍵} ⋄ f←${shape} ⋄ f ${n}`); return error ?? Number(outputs.at(-1)); };
    parentPort.postMessage(Object.fromEntries(Object.entries(shapes).map(([name, [shape, n]]) => [name, [depth(shape, n), depth(shape, 300)]])));
}
