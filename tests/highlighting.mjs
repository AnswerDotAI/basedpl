import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHighlighterCore} from 'shiki/core';
import {createJavaScriptRegexEngine} from 'shiki/engine/javascript';
const grammar = JSON.parse(readFileSync(new URL('../python/basedpl/bpl.tmLanguage.json', import.meta.url), 'utf8'));
const hanzi = JSON.parse(readFileSync(new URL('../python/basedpl/hpl.tmLanguage.json', import.meta.url), 'utf8'));
const theme = {name: 'test', settings: [
    {scope: 'keyword.operator.monadic', settings: {foreground: 'var(--quarto-hl-op-color)'}},
    {scope: 'variable.other', settings: {foreground: 'var(--bpl-name-color)'}}]};
const highlight = await createHighlighterCore({langs: [grammar, hanzi], themes: [theme], engine: createJavaScriptRegexEngine()});
function scopes(text, lang = 'bpl') {
    return highlight.codeToTokensBase(text, {lang, theme: 'test', includeExplanation: 'scopeName'}).flatMap(line =>
        line.flatMap(token => token.explanation.map(part => [part.content, part.scopes.at(-1).scopeName])));
}
for (const lang of ['bpl', 'hpl']) for (const text of ['"a""b"', '"加""总"', "''", "'''", "'a'"])
    assert.ok(scopes(text, lang).every(([, scope]) => scope.startsWith('string.quoted.')));
assert.ok(scopes('⍝ "ignored" +ₓ').every(([, scope]) => scope === 'comment.line.bpl'));
assert.ok(scopes('注 "ignored" 加ₓ', 'hpl').every(([, scope]) => scope === 'comment.line.bpl'));
assert.deepEqual(scopes(']ₓ+'), [[']', 'source.bpl'], ['ₓ', 'constant.numeric.bpl'], ['+', 'support.function.bpl']]);
assert.deepEqual(scopes('xⱼy+nₓ').map(([, scope]) => scope), ['variable.other.bpl', 'constant.numeric.bpl', 'variable.other.bpl', 'support.function.bpl', 'variable.other.bpl',
    'keyword.operator.monadic.bpl']);
for (const lang of ['bpl', 'hpl']) for (const name of ['avg', 'x_12', '测试', '😎', '_+', '_加', '_加_总', 'x_語'])
    assert.deepEqual(scopes(name, lang), [[name, 'variable.other.bpl']]);
for (const name of ['加总内乙', '加總內乙', '是']) assert.deepEqual(scopes(name), [[name, 'variable.other.bpl']]);
for (const text of ['¯2ₓ', '1ᵣ3', '1ₑ¯2', '2ⱼ¯4'])
    assert.ok(scopes(text).every(([, scope]) => scope === 'constant.numeric.bpl'));
for (const text of ['负2ₓ', '負永', '1ₑ负2', '2j負4', '空ₓ', '1内2', '1內2', '内5'])
    assert.ok(scopes(text, 'hpl').every(([, scope]) => scope === 'constant.numeric.bpl'));
assert.deepEqual(scopes('+/@⍵²₀').map(([, scope]) => scope), ['support.function.bpl', 'keyword.operator.monadic.bpl',
    'keyword.operator.dyadic.bpl', 'variable.parameter.bpl', 'keyword.operator.monadic.bpl', 'keyword.control.bpl']);
assert.deepEqual(scopes('+/@⍵²₀', 'hpl'), scopes('+/@⍵²₀'));
for (const text of ['加总内乙', '加總內乙']) assert.deepEqual(scopes(text, 'hpl').map(([, scope]) => scope),
    ['support.function.bpl', 'keyword.operator.monadic.bpl', 'keyword.operator.dyadic.bpl', 'variable.parameter.bpl']);
assert.ok(scopes('系prefs', 'hpl').every(([, scope]) => scope === 'support.function.bpl'));
assert.deepEqual(scopes('mean是加总除数', 'hpl').map(([part]) => part), ['mean', '是', '加', '总', '除', '数']);
assert.equal(scopes('"open\n+').at(-1)[1], 'support.function.bpl');
assert.equal(scopes('⍝ comment\n+').at(-1)[1], 'support.function.bpl');
assert.ok(highlight.codeToHtml('+/2', {lang: 'bpl', theme: 'test'}).includes('color:var(--quarto-hl-op-color)'));
