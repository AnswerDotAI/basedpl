import {grammar, symbols} from './language.js';
import input from './input.js';
import languageBar from './lb.js';
import layout from './layout.json' with {type: 'json'};

export {grammar};
export const configuration = {
    comments: {lineComment: '⍝'}, brackets: [['(', ')'], ['[', ']'], ['{', '}']],
    autoClosingPairs: [['(', ')'], ['[', ']'], ['{', '}'], ['"', '"'], ["'", "'"]].map(([open, close]) => ({open, close, notIn: ['string', 'comment']})),
    surroundingPairs: [['(', ')'], ['[', ']'], ['{', '}'], ['"', '"'], ["'", "'"]].map(([open, close]) => ({open, close})),
};
export const editorOptions = {fontFamily: 'SAX2B, monospace', unicodeHighlight: {ambiguousCharacters: false}};
const font = new URL('./SAX2B.woff2', import.meta.url);
const mount = languageBar(symbols, input, layout, editorOptions);

export function installFont(doc = document) {
    if (doc.getElementById('bpl-font')) return;
    const style = doc.createElement('style');
    style.id = 'bpl-font';
    style.textContent = `@font-face{font-family:SAX2B;src:url("${font}") format("woff2")}`;
    doc.head.append(style);
}

export function install() {
    if (window.bplInput?.element.isConnected) { window.bplInput.fonts(); return window.bplInput; }
    window.bplInput?.dispose();
    installFont();
    const container = document.getElementById('bpl-toolbar');
    window.bplInput = mount(container || document.body);
    if (!container) window.bplInput.element.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:2147483647';
    return window.bplInput;
}
