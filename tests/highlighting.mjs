import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHighlighterCore} from 'shiki/core';
import {createJavaScriptRegexEngine} from 'shiki/engine/javascript';
const grammar = JSON.parse(readFileSync(new URL('../python/basedpl/bpl.tmLanguage.json', import.meta.url), 'utf8'));
const theme = {name: 'test', settings: [
    {scope: 'keyword.operator.monadic', settings: {foreground: 'var(--quarto-hl-op-color)'}},
    {scope: 'variable.other', settings: {foreground: 'var(--bpl-name-color)'}}]};
const highlight = await createHighlighterCore({langs: [grammar], themes: [theme], engine: createJavaScriptRegexEngine()});
function scopes(text) {
    return highlight.codeToTokensBase(text, {lang: 'bpl', theme: 'test', includeExplanation: 'scopeName'}).flatMap(line =>
        line.flatMap(token => token.explanation.map(part => [part.content, part.scopes.at(-1).scopeName])));
}
for (const text of ['"a""b"', "''", "'''", "'a'"])
    assert.ok(scopes(text).every(([, scope]) => scope.startsWith('string.quoted.')));
assert.ok(scopes('⍝ "ignored" +ₓ').every(([, scope]) => scope === 'comment.line.bpl'));
assert.ok(scopes('注 "ignored" 加ₓ').every(([, scope]) => scope === 'comment.line.bpl'));
assert.deepEqual(scopes(']ₓ+'), [[']', 'source.bpl'], ['ₓ', 'constant.numeric.bpl'], ['+', 'support.function.bpl']]);
assert.deepEqual(scopes('xⱼy+nₓ').map(([, scope]) => scope), ['variable.other.bpl', 'constant.numeric.bpl', 'variable.other.bpl', 'support.function.bpl', 'variable.other.bpl',
    'keyword.operator.monadic.bpl']);
for (const name of ['avg', 'x_12', '测试', '😎']) assert.deepEqual(scopes(name), [[name, 'variable.other.bpl']]);
for (const text of ['¯2ₓ', '1ᵣ3', '1ₑ¯2', '2ⱼ¯4', '负2ₓ', '負永', '1ₑ负2', '2j負4', '空ₓ', '1内2', '1內2', '内5'])
    assert.ok(scopes(text).every(([, scope]) => scope === 'constant.numeric.bpl'));
assert.deepEqual(scopes('+/@⍵²₀').map(([, scope]) => scope), ['support.function.bpl', 'keyword.operator.monadic.bpl',
    'keyword.operator.dyadic.bpl', 'variable.parameter.bpl', 'keyword.operator.monadic.bpl', 'keyword.control.bpl']);
for (const text of ['加总内乙', '加總內乙']) assert.deepEqual(scopes(text).map(([, scope]) => scope),
    ['support.function.bpl', 'keyword.operator.monadic.bpl', 'keyword.operator.dyadic.bpl', 'variable.parameter.bpl']);
assert.ok(scopes('系prefs').every(([, scope]) => scope === 'support.function.system.bpl'));
assert.deepEqual(scopes('是'), [['是', 'keyword.control.bpl']]);
assert.equal(scopes('"open\n+').at(-1)[1], 'support.function.bpl');
assert.equal(scopes('⍝ comment\n+').at(-1)[1], 'support.function.bpl');
assert.ok(highlight.codeToHtml('+/2', {lang: 'bpl', theme: 'test'}).includes('color:var(--quarto-hl-op-color)'));
