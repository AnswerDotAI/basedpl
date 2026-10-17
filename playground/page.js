// The playground page: a REPL. It runs the editor's code in a worker and shows each output under that code as it arrives. Stop ends
// the session and starts a new one.
import {display, editor, highlight, interpreter, show} from './bpl.js';
const code = document.querySelector('#code'), history = document.querySelector('#history');
const run = document.querySelector('#run'), stop = document.querySelector('#stop');
const input = await editor(code, go, {
    value: 'avg←+/÷≢ ⋄ avg 2 4 9', lineNumbers: 'off', lineDecorationsWidth: 0, scrollbar: {vertical: 'hidden', horizontal: 'hidden'}
});
function resize() { code.style.height = `${input.getContentHeight()}px`; input.layout(); }
input.onDidContentSizeChange(e => { if (e.contentHeightChanged) resize(); });
resize();
let busy = false, out = history;
const bpl = interpreter(data => {
    const rect = code.getBoundingClientRect(), following = rect.top >= 0 && rect.bottom <= innerHeight;
    display(out, data);
    if (data.type === 'reply' || data.type === 'crash') finish();
    if (following) code.scrollIntoView({block: 'nearest'});
});

function finish() { busy = false; run.disabled = false; stop.disabled = true; }

function go() {
    if (busy) return;
    const text = input.getValue();
    input.setValue('');
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
    bpl.run(text);
    input.focus();
    code.scrollIntoView({block: 'nearest'});
}

run.onclick = go;
stop.onclick = () => { bpl.restart(); show(out, 'pre', 'Stopped. Started a new session.', 'error'); finish(); };
finish();
