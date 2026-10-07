// The playground page. It runs the textarea's code in a worker, shows each output as it arrives, and adds the language bar. Stop ends
// the worker and starts a new one, as a trap does.
import {createHighlighterCore} from 'https://esm.sh/shiki@4.5.0/core?bundle';
import {createJavaScriptRegexEngine} from 'https://esm.sh/shiki@4.5.0/engine/javascript?bundle';
const grammar = await (await fetch(new URL('bpl.tmLanguage.json', import.meta.url))).json();
const classes = [['comment', 'co'], ['string.quoted.double', 'st'], ['string.quoted.single', 'ch'], ['constant.numeric', 'dv'],
    ['support.function', 'fu'], ['support.function.system', 'bu'], ['keyword.operator.monadic', 'op'],
    ['keyword.operator.dyadic', 'ex'], ['variable.parameter', 'va'], ['keyword.control', 'kw']];
const highlight = await createHighlighterCore({langs: [grammar], engine: createJavaScriptRegexEngine(), themes: [{
    name: 'quarto', fg: 'var(--quarto-hl-kw-color)', bg: 'transparent',
    settings: classes.map(([scope, cls]) => ({scope, settings: {foreground: `var(--quarto-hl-${cls}-color)`, fontStyle: cls === 'kw' ? 'bold' : ''}}))
}]});
const code = document.querySelector('#code'), history = document.querySelector('#history');
const run = document.querySelector('#run'), stop = document.querySelector('#stop');
let worker, busy = false, bar = false, out = history;

function show(tag, text, cls) {
    const el = out.appendChild(document.createElement(tag));
    if (cls) el.className = cls;
    if (text !== undefined) (tag === 'pre' ? el.appendChild(document.createElement('code')) : el).textContent = text;
    return el;
}

// One output: a MIME bundle, shown in its richest form. Binary types arrive as base64.
function render({ kind, data }) {
    if (data['image/svg+xml'] || data['text/html']) show('div').innerHTML = data['image/svg+xml'] ?? data['text/html'];
    else if (data['image/png'] || data['image/jpeg']) {
        const type = data['image/png'] ? 'image/png' : 'image/jpeg';
        show('img').src = `data:${type};base64,${data[type]}`;
    } else if (kind === 'text' && out.lastElementChild?.matches('pre.text')) out.lastElementChild.firstElementChild.textContent += data['text/plain'];
    else show('pre', data['text/plain'], kind === 'text' ? 'text' : '');
}

function finish() { busy = false; run.disabled = false; stop.disabled = true; }

function start() {
    worker = new Worker(new URL('worker.js', import.meta.url), { type: 'module' });
    worker.onmessage = ({ data }) => {
        const rect = code.getBoundingClientRect(), following = rect.top >= 0 && rect.bottom <= innerHeight;
        if (data.type === 'ready') {
            if (!bar) addBar(data.symbols);
            run.disabled = false;
        } else if (data.type === 'output') render(data.output);
        else if (data.type === 'reply') {
            if (data.reply.error) show('pre', data.reply.error.display, 'error');
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
    const text = name => fetch(new URL(name, import.meta.url)).then(r => r.text());
    const [lb, input, layout] = await Promise.all([text('lb.js'), text('input.js'), text('layout.json')]);
    (0, eval)(lb)(symbols, (0, eval)(input), JSON.parse(layout));
}

function go() {
    if (busy || run.disabled) return;
    const text = code.value;
    code.value = '';
    busy = true;
    run.disabled = true;
    stop.disabled = false;
    const cell = history.appendChild(document.createElement('div'));
    cell.className = 'cell';
    const source = cell.appendChild(document.createElement('div'));
    source.className = 'sourceCode';
    source.innerHTML = highlight.codeToHtml(text, {lang: 'bpl', theme: 'quarto', transformers: [{
        pre(node) { this.addClassToHast(node, 'sourceCode bpl'); delete node.properties.style; },
        code(node) { this.addClassToHast(node, 'sourceCode bpl'); }
    }]});
    out = cell.appendChild(document.createElement('div'));
    out.className = 'cell-output cell-output-display';
    worker.postMessage(text);
    code.scrollIntoView({block: 'nearest'});
}

run.onclick = go;
code.addEventListener('keydown', e => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); go(); } });
stop.onclick = () => restart('Stopped. Started a new session.');
start();
