// The playground page. It runs the editor's code in a worker, shows each output as it arrives, and adds the language bar. Stop ends
// the worker and starts a new one, as a trap does.
// esm.sh's Node process shim masks the browser's OS; jsDelivr preserves Monaco's platform detection.
import * as monaco from 'https://cdn.jsdelivr.net/npm/monaco-editor-core@0.57.0/+esm';
// Unbundled Shiki imports share TextMate's state singleton with the Monaco integration.
import {shikiToMonaco} from 'https://esm.sh/@shikijs/monaco@4.5.0';
import {createHighlighterCore} from 'https://esm.sh/shiki@4.5.0/core';
import {createJavaScriptRegexEngine} from 'https://esm.sh/shiki@4.5.0/engine/javascript';
const code = document.querySelector('#code'), history = document.querySelector('#history');
const run = document.querySelector('#run'), stop = document.querySelector('#stop');
const css = getComputedStyle(code), color = cls => {
    const value = css.getPropertyValue(`--quarto-hl-${cls}-color`).trim();
    return value === 'inherit' ? color('kw') : value;
};
// The npm package's files come from the copies of your `cargo wasm` build beside this script on localhost, and from the latest
// release elsewhere. The worker takes this base URL as its `pkg` parameter.
const pkg = location.hostname === 'localhost' ? new URL('./', import.meta.url).href : 'https://cdn.jsdelivr.net/npm/basedpl/';
const grammar = await (await fetch(new URL('bpl.tmLanguage.json', pkg))).json();
const classes = [['comment', 'co'], ['string.quoted.double', 'st'], ['string.quoted.single', 'ch'], ['constant.numeric', 'dv'],
    ['support.function', 'fu'], ['support.function.system', 'bu'], ['keyword.operator.monadic', 'op'],
    ['keyword.operator.dyadic', 'ex'], ['variable.parameter', 'va'], ['keyword.control', 'kw']];
const highlight = await createHighlighterCore({langs: [grammar], engine: createJavaScriptRegexEngine(), themes: [{
    name: 'quarto', type: 'light', fg: color('kw'), bg: css.getPropertyValue('--bs-body-bg').trim(),
    settings: classes.map(([scope, cls]) => ({scope, settings: {foreground: color(cls), fontStyle: cls === 'kw' ? 'bold' : ''}}))
}]});
window.monaco = monaco;
window.MonacoEnvironment = {getWorker: () => new Worker(new URL('monaco-worker.js', import.meta.url), {type: 'module'})};
monaco.languages.register({id: 'bpl'});
shikiToMonaco(highlight, monaco);
await document.fonts.load(`${css.fontSize} ${css.fontFamily}`);
const editor = monaco.editor.create(code, {
    value: 'avg←+/÷≢ ⋄ avg 2 4 9', language: 'bpl', theme: 'quarto', ariaLabel: 'BPL code',
    fontFamily: css.fontFamily, fontSize: parseFloat(css.fontSize), lineHeight: parseFloat(css.lineHeight),
    wordWrap: 'on', scrollBeyondLastLine: false, automaticLayout: true, padding: {top: 4, bottom: 4},
    minimap: {enabled: false}, lineNumbers: 'off', folding: false, lineDecorationsWidth: 0,
    overviewRulerLanes: 0, renderLineHighlight: 'none', scrollbar: {vertical: 'hidden', horizontal: 'hidden'},
    quickSuggestions: false, wordBasedSuggestions: 'off'
});
function resize() { code.style.height = `${editor.getContentHeight()}px`; editor.layout(); }
editor.onDidContentSizeChange(e => { if (e.contentHeightChanged) resize(); });
resize();
let worker, busy = false, bar = false, out = history;

function show(tag, text, cls) {
    const el = out.appendChild(document.createElement(tag));
    if (cls) el.className = cls;
    if (text !== undefined) (tag === 'pre' ? el.appendChild(document.createElement('code')) : el).textContent = text;
    return el;
}

// One output: a MIME bundle, shown in its richest form. Binary types arrive as `Uint8Array`s. `image/x-rgba` holds RGBA pixels, with
// the width as a parameter.
function render({ kind, data }) {
    const rgba = Object.keys(data).find(type => type.startsWith('image/x-rgba;'));
    if (data['image/svg+xml'] || data['text/html']) show('div').innerHTML = data['image/svg+xml'] ?? data['text/html'];
    else if (rgba) {
        const image = new ImageData(new Uint8ClampedArray(data[rgba].buffer), +rgba.split('width=')[1]);
        Object.assign(show('canvas'), { width: image.width, height: image.height }).getContext('2d').putImageData(image, 0, 0);
    } else if (data['image/png'] || data['image/jpeg']) {
        const type = data['image/png'] ? 'image/png' : 'image/jpeg';
        Object.assign(show('img'), { onload() { URL.revokeObjectURL(this.src); }, src: URL.createObjectURL(new Blob([data[type]], { type })) });
    } else if (kind === 'text' && out.lastElementChild?.matches('pre.text')) out.lastElementChild.firstElementChild.textContent += data['text/plain'];
    else show('pre', data['text/plain'], kind === 'text' ? 'text' : '');
}

function finish() { busy = false; run.disabled = false; stop.disabled = true; }

function start() {
    worker = new Worker(new URL(`worker.js?pkg=${encodeURIComponent(pkg)}`, import.meta.url), { type: 'module' });
    worker.onmessage = ({ data }) => {
        const rect = code.getBoundingClientRect(), following = rect.top >= 0 && rect.bottom <= innerHeight;
        if (data.type === 'ready') {
            if (!bar) addBar(data.symbols);
            run.disabled = false;
        } else if (data.type === 'output') render(data.output);
        else if (data.type === 'reply') {
            if (data.error) show('pre', data.error.display, 'error');
            finish();
        } else if (data.type === 'panic') show('pre', `BPL panicked: ${data.message}`, 'error');
        else if (data.type === 'crash') restart(`The interpreter stopped (${data.message}). Started a new session.`);
        if (following) code.scrollIntoView({block: 'nearest'});
    };
}

function restart(message) {
    worker.terminate();
    show('pre', message, 'error');
    finish();
    run.disabled = true;
    start();
}

// The language bar from `lb.js`, which needs the glyph rows, `input.js` and the key layout.
async function addBar(symbols) {
    bar = true;
    const text = name => fetch(new URL(name, pkg)).then(r => r.text());
    const [lb, input, layout] = await Promise.all([text('lb.js'), text('input.js'), text('layout.json')]);
    (0, eval)(lb)(symbols, (0, eval)(input), JSON.parse(layout));
}

function go() {
    if (busy || run.disabled) return;
    const text = editor.getValue();
    editor.setValue('');
    busy = true;
    run.disabled = true;
    stop.disabled = false;
    const cell = history.appendChild(document.createElement('div'));
    cell.className = 'cell';
    const scaffold = cell.appendChild(document.createElement('div'));
    scaffold.className = 'code-copy-outer-scaffold';
    const source = scaffold.appendChild(document.createElement('div'));
    source.className = 'sourceCode';
    source.innerHTML = highlight.codeToHtml(text, {lang: 'bpl', theme: 'quarto', transformers: [{
        pre(node) { this.addClassToHast(node, 'sourceCode bpl code-with-copy'); delete node.properties.style; },
        code(node) { this.addClassToHast(node, 'sourceCode bpl'); }
    }]});
    scaffold.insertAdjacentHTML('beforeend', '<button type="button" class="code-copy-button" title="Copy to Clipboard" aria-label="Copy to Clipboard"><i class="bi" aria-hidden="true"></i></button>');
    out = cell.appendChild(document.createElement('div'));
    out.className = 'cell-output cell-output-display';
    worker.postMessage(text);
    editor.focus();
    code.scrollIntoView({block: 'nearest'});
}

run.onclick = go;
editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, go);
editor.addCommand(monaco.KeyMod.WinCtrl | monaco.KeyCode.Enter, go);
stop.onclick = () => restart('Stopped. Started a new session.');
start();
