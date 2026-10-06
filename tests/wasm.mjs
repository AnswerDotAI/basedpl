// Checks the browser module, built by `cargo wasm`, in Node. The depth checks run in a worker with a stack about the size of a Chrome
// worker's. The call limit allows 200 levels of recursion that makes two calls a level, and 135 of recursion that makes three. Each
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
    const { error } = JSON.parse(new Session().request(JSON.stringify({ code }), o => outputs.push(JSON.parse(o).data['text/plain'])));
    return { outputs, error: error?.kind };
};

// Each shape, and the depth it must reach.
const shapes = {
    plain: ['{⍵=0?0;1+∇ ⍵-1}', 200],
    assigned: ['{⍵=0?0;x←∇ ⍵-1 ⋄ x+1}', 200],
    each: ['{⍵=0?0;1+↑f¨⍵-1}', 200],
    rank: ['{⍵=0?0;1+↑f⍤0⊢⍵-1}', 200],
    operand: ['{⍵=0?0;1+(f m) ⍵-1}', 200],
    outer: ['{⍵=0?0;1+↑(0 f⊗ ⍵-1)}', 135],
};

if (isMainThread) {
    test('runs code and streams its output', () => {
        assert.deepEqual(run('⎕←"hi" ⋄ +/⍳4'), { outputs: ['hi', '6'], error: undefined });
    });
    test('matches with JavaScript regular expressions and templates, at character positions', () => {
        const { outputs } = run('r←•r "(?<=é|\\s)(?<x>[bB])(\\d)?" ⋄ m←r.matches "éb1 B" ⋄ ⎕←m.position ⋄ ⎕←"<$<x>$2>" r.replace "éb1 B" ⋄ m.groups');
        assert.deepEqual(outputs, ['[1 4]ₓ', 'é<b1> <B>', '("b" "1" ⋄ "B" "")']);
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
