// Run with: node tests/highlighting.mjs /path/to/node_modules/shiki/dist
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
const shiki = pathToFileURL(process.argv[2] + '/');
const {createHighlighterCore} = await import(new URL('core.mjs', shiki));
const {createJavaScriptRegexEngine} = await import(new URL('engine-javascript.mjs', shiki));
const grammar = JSON.parse(readFileSync(new URL('../python/basedpl/bpl.tmLanguage.json', import.meta.url), 'utf8'));
const theme = {name: 'test', settings: [{scope: 'keyword.operator.monadic', settings: {foreground: 'var(--quarto-hl-op-color)'}}]};
const highlight = await createHighlighterCore({langs: [grammar], themes: [theme], engine: createJavaScriptRegexEngine()});
function scopes(text) {
    return highlight.codeToTokensBase(text, {lang: 'bpl', theme: 'test', includeExplanation: 'scopeName'}).flatMap(line =>
        line.flatMap(token => token.explanation.map(part => [part.content, part.scopes.at(-1).scopeName])));
}
for (const text of ['"a""b"', "''", "'''", "'a'"])
    assert.ok(scopes(text).every(([, scope]) => scope.startsWith('string.quoted.')));
assert.ok(scopes('⍝ "ignored" +ₓ').every(([, scope]) => scope === 'comment.line.bpl'));
assert.deepEqual(scopes(']ₓ+'), [[']', 'source.bpl'], ['ₓ', 'constant.numeric.bpl'], ['+', 'support.function.bpl']]);
for (const text of ['¯2ₓ', '1ᵣ3', '1ₑ¯2', '2ⱼ¯4']) assert.ok(scopes(text).every(([, scope]) => scope === 'constant.numeric.bpl'));
assert.deepEqual(scopes('+/⌾⍵²₀').map(([, scope]) => scope), ['support.function.bpl', 'keyword.operator.monadic.bpl',
    'keyword.operator.dyadic.bpl', 'variable.parameter.bpl', 'keyword.operator.monadic.bpl', 'keyword.control.bpl']);
assert.equal(scopes('"open\n+').at(-1)[1], 'support.function.bpl');
assert.equal(scopes('⍝ comment\n+').at(-1)[1], 'support.function.bpl');
assert.ok(highlight.codeToHtml('+/2', {lang: 'bpl', theme: 'test'}).includes('color:var(--quarto-hl-op-color)'));
