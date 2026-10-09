// What the playground and the gallery share: a Monaco editor for BPL with its highlighting and language bar, a worker that runs BPL,
// and the display of what the worker reports.
// esm.sh's Node process shim masks the browser's OS; jsDelivr preserves Monaco's platform detection.
import * as monaco from 'https://cdn.jsdelivr.net/npm/monaco-editor-core@0.57.0/+esm';
// Unbundled Shiki imports share TextMate's state singleton with the Monaco integration.
import {shikiToMonaco} from 'https://esm.sh/@shikijs/monaco@4.5.0';
import {createHighlighterCore} from 'https://esm.sh/shiki@4.5.0/core';
import {createJavaScriptRegexEngine} from 'https://esm.sh/shiki@4.5.0/engine/javascript';

// The npm package's files come from the copies of your `cargo wasm` build beside this script on localhost, and from the latest
// release elsewhere, resolved once so every asset uses the same version. The worker takes this base URL as its `pkg` parameter.
let pkg = new URL('./', import.meta.url).href;
if (location.hostname !== 'localhost') {
    const response = await fetch('https://cdn.jsdelivr.net/npm/basedpl/package.json', {cache: 'no-cache'});
    const {version} = await response.json();
    pkg = `https://cdn.jsdelivr.net/npm/basedpl@${version}/`;
}
// The text of the package file `name`, and the value of the script in it.
const text = name => fetch(new URL(name, pkg)).then(r => r.text());
const script = async name => (0, eval)(await text(name));
document.head.insertAdjacentHTML('beforeend', '<link rel="stylesheet" href="https://esm.sh/monaco-editor-core@0.57.0/es2022/monaco-editor-core.css">');
const css = getComputedStyle(document.documentElement), color = cls => {
    const value = css.getPropertyValue(`--quarto-hl-${cls}-color`).trim();
    return value === 'inherit' ? color('kw') : value;
};
const grammar = JSON.parse(await text('bpl.tmLanguage.json'));
const classes = [['comment', 'co'], ['string.quoted.double', 'st'], ['string.quoted.single', 'ch'], ['constant.numeric', 'dv'],
    ['support.function', 'fu'], ['support.function.system', 'bu'], ['keyword.operator.monadic', 'op'],
    ['keyword.operator.dyadic', 'ex'], ['variable.parameter', 'va'], ['keyword.control', 'kw']];
export const highlight = await createHighlighterCore({langs: [grammar], engine: createJavaScriptRegexEngine(), themes: [{
    name: 'quarto', type: 'light', fg: color('kw'), bg: css.getPropertyValue('--bs-body-bg').trim(),
    settings: classes.map(([scope, cls]) => ({scope, settings: {foreground: color(cls), fontStyle: cls === 'kw' ? 'bold' : ''}}))
}]});
// `lb.js` finds the page's Monaco editors through `window.monaco`.
window.monaco = monaco;
window.MonacoEnvironment = {getWorker: () => new Worker(new URL('monaco-worker.js', import.meta.url), {type: 'module'})};
monaco.languages.register({id: 'bpl'});
shikiToMonaco(highlight, monaco);

// A Monaco editor for BPL in `element`, in the element's font. Ctrl-Enter or Cmd-Enter calls `run`. `options` adds to the defaults.
export async function editor(element, run, options) {
    const font = getComputedStyle(element);
    await document.fonts.load(`${font.fontSize} ${font.fontFamily}`);
    const e = monaco.editor.create(element, {
        language: 'bpl', theme: 'quarto', ariaLabel: 'BPL code', automaticLayout: true, padding: {top: 4, bottom: 4},
        fontFamily: font.fontFamily, fontSize: parseFloat(font.fontSize), lineHeight: parseFloat(font.lineHeight),
        wordWrap: 'on', scrollBeyondLastLine: false, minimap: {enabled: false}, folding: false, overviewRulerLanes: 0,
        renderLineHighlight: 'none', quickSuggestions: false, wordBasedSuggestions: 'off',
        unicodeHighlight: {ambiguousCharacters: false}, ...options
    });
    e.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, run);
    e.addCommand(monaco.KeyMod.WinCtrl | monaco.KeyCode.Enter, run);
    return e;
}

// Adds a `tag` element to `out`, with class `cls` and `text`. A `pre` holds its text in a `code` element.
export function show(out, tag, text, cls) {
    const el = out.appendChild(document.createElement(tag));
    if (cls) el.className = cls;
    if (text !== undefined) (tag === 'pre' ? el.appendChild(document.createElement('code')) : el).textContent = text;
    return el;
}

let canvasScript;
// Draws a `•canvas` output with the package's `canvas.js`, which the page loads when it first needs it.
async function drawCanvas(out, canvas, json, bytes) {
    canvasScript ??= script('canvas.js');
    try { (await canvasScript)(canvas, json, bytes); } catch (e) { show(out, 'pre', String(e), 'error'); }
}

// One output: a MIME bundle, shown in its richest form. Binary types arrive as `Uint8Array`s. A `•canvas` drawing comes as JSON with
// a second entry of bytes. `image/x-rgba` holds RGBA pixels, with the width as a parameter.
function render(out, { kind, data }) {
    const rgba = Object.keys(data).find(type => type.startsWith('image/x-rgba;'));
    const canvas = data['application/x-bpl-canvas+json'];
    if (canvas) drawCanvas(out, show(out, 'canvas'), canvas, data['application/x-bpl-canvas-data']);
    // A parsed fragment runs the scripts in the markup, which `innerHTML` doesn't.
    else if (data['image/svg+xml'] || data['text/html']) show(out, 'div').replaceChildren(document.createRange().createContextualFragment(data['image/svg+xml'] ?? data['text/html']));
    else if (rgba) {
        const image = new ImageData(new Uint8ClampedArray(data[rgba].buffer), +rgba.split('width=')[1]);
        Object.assign(show(out, 'canvas'), { width: image.width, height: image.height }).getContext('2d').putImageData(image, 0, 0);
    } else if (data['image/png'] || data['image/jpeg']) {
        const type = data['image/png'] ? 'image/png' : 'image/jpeg';
        Object.assign(show(out, 'img'), { onload() { URL.revokeObjectURL(this.src); }, src: URL.createObjectURL(new Blob([data[type]], { type })) });
    } else if (kind === 'text' && out.lastElementChild?.matches('pre.text')) out.lastElementChild.firstElementChild.textContent += data['text/plain'];
    else show(out, 'pre', data['text/plain'], kind === 'text' ? 'text' : '');
}

// Shows in `out` what a message from the worker reports: an output, an error, a panic or a trap.
export function display(out, data) {
    if (data.type === 'output') render(out, data.output);
    else if (data.type === 'reply' && data.error) show(out, 'pre', data.error.display, 'error');
    else if (data.type === 'panic') show(out, 'pre', `BPL panicked: ${data.message}`, 'error');
    else if (data.type === 'crash') show(out, 'pre', `The interpreter stopped (${data.message}). Started a new session.`, 'error');
}

let bar;
// The language bar from `lb.js`, which needs the glyph rows, `input.js` and the key layout.
async function addBar(symbols) {
    const [lb, input, layout] = await Promise.all([script('lb.js'), script('input.js'), text('layout.json')]);
    lb(symbols, input, JSON.parse(layout));
}

// Runs BPL in a worker, which holds one session. `run(code)` runs code, and `on` gets each message about it: `output`, then `reply` with
// the error or `null`. `eval(code)` returns a promise of the code's value, which rejects with the BPL error. `on` also gets `panic` and
// `crash`. A trap leaves the worker unusable, so a `crash` also starts a new one. Messages sent before the worker is ready wait for it.
// A restart drops them, and the promises of unanswered evals never settle. The first worker to start adds the language bar.
export function interpreter(on) {
    // `asked` holds, in sending order, `null` for each run and the promise's settlers for each eval, until the worker answers it.
    let worker, ready, waiting, asked;
    function start() {
        [ready, waiting, asked] = [false, [], []];
        worker = new Worker(new URL(`worker.js?pkg=${encodeURIComponent(pkg)}`, import.meta.url), { type: 'module' });
        worker.onmessage = ({ data }) => {
            if (data.type === 'ready') {
                bar ??= addBar(data.symbols);
                ready = true;
                for (const message of waiting) worker.postMessage(message);
                return;
            }
            const promise = data.type === 'reply' || data.type === 'value' ? asked.shift() : null;
            if (promise) data.type === 'value' ? promise.resolve(data.value) : promise.reject(data.error);
            else on(data);
            if (data.type === 'crash') restart();
        };
    }
    const send = message => { if (ready) worker.postMessage(message); else waiting.push(message); };
    // Ends the session, and any code it is running, and starts a new one.
    function restart() { worker.terminate(); start(); }
    start();
    return {
        run(code) { asked.push(null); send(code); },
        eval: code => new Promise((resolve, reject) => { asked.push({resolve, reject}); send({eval: code}); }),
        restart
    };
}
