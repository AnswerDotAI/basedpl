import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const root = new URL('../python/basedpl/', import.meta.url);
const layout = JSON.parse(readFileSync(new URL('layout.json', root), 'utf8'));
const {press, inCode, entry} = eval(readFileSync(new URL('input.js', root), 'utf8'))(
    [{glyph: '⍝', kind: 'comment', hanzi: ['注', '注']}], layout);
for (const text of ["''", "'''"]) assert.equal(inCode(text), true);
for (const marker of ['⍝', '注']) {
    assert.equal(inCode(`${marker} \"ignored\" +`), false);
    assert.equal(inCode(`\"${marker}\"`), true);
    assert.equal(inCode(`${marker} comment\n加`), true);
    const text = `${marker} \`sum`;
    assert.equal(entry({bpl: true, text, pos: text.length, empty: true}), undefined);
}
for (const key of ['£', '‹']) assert.deepEqual(press({key, code: 'Digit3', shiftKey: true, altKey: true}, true), {text: '#', stop: true});
for (const [code, key, text] of [['Backslash', '\\', '⍭'], ['Digit6', '§', '⁶']]) {
    const ev = {code, key, shiftKey: false, altKey: true};
    assert.deepEqual(press(ev, true), {text: '', stop: true});
    assert.deepEqual(press(ev, true), {text, stop: true});
    assert.equal(press({key: '6', code: 'Digit6'}, true), undefined);
}
assert.deepEqual(press({key: 'ø', code: 'KeyO', altKey: true}, true), {text: '', stop: true});
assert.deepEqual(press({key: 'o', code: 'KeyO'}, true), {text: '○', stop: true});
let composed = '';
for (const [code, key] of [['Digit5', '5'], ['KeyX', 'x'], ['Digit5', '5'], ['Digit1', '1']])
    composed += press({code, key, altKey: true}, true).text;
assert.equal(composed, 'ₓ₁');
for (const [prefix, text] of [['5', '₆'], ['6', '⁶']]) {
    press({code: `Digit${prefix}`, key: prefix, altKey: true}, true);
    assert.deepEqual(press({code: 'Digit6', key: '^', shiftKey: true, altKey: true}, true), {text, stop: true});
}
