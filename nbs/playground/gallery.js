// The gallery page. The nav links' fragments name the demos, in display order. The page reads each demo's BPL from `gallery/`,
// labels its link with the demo's title, and shows the chosen demo's description under the output and its code in the editor.
// Choosing a demo runs it. Run runs the edited code, and Reset brings back the demo's own code and runs it. Each run starts a new
// session.
//
// A demo that defines a function `frame` is an animation. Once the demo has run, each animation tick draws `frame 0`, `frame 1` and
// so on with the canvas driver, `driver.js`, on a 500 × 500 canvas. An `fps` option in the demo's `options` record sets the rate,
// and the driver gets the other options. Without `fps`, each tick draws the next frame. The animation stops when another run
// starts, when a frame raises an error, or when `frame` gives no value.
import {display, editor, interpreter, show} from './bpl.js';
const code = document.querySelector('#code'), out = document.querySelector('#output'), about = document.querySelector('#about');
const links = [...document.querySelectorAll('#demos a')], outputTab = document.querySelector('[data-bs-target="#gallery-output"]');
const input = await editor(code, go, {lineNumbersMinChars: 3});
// Each demo file starts with `⍝` lines. The first is the demo's title, which labels its link. The rest describe it.
const demos = Object.fromEntries(await Promise.all(links.map(async a => {
    const lines = (await (await fetch(`gallery/${a.hash.slice(1)}.bpl`, {cache: 'no-cache'})).text()).split('\n');
    const n = lines.findIndex(line => !line.startsWith('⍝'));
    const [title, ...text] = lines.slice(0, n).map(line => line.replace(/^⍝ ?/, ''));
    a.textContent = title;
    return [a.hash.slice(1), {about: text.join(' '), code: lines.slice(n).join('\n')}];
})));
const bpl = interpreter(data => {
    display(out, data);
    if (data.type === 'reply' && !data.error) animate(runs);
});
const tick = () => new Promise(requestAnimationFrame);
let runs = 0, demo, driver;

function go() {
    if (runs++) bpl.restart();
    out.replaceChildren();
    outputTab.click();
    bpl.run(input.getValue());
}

// Animates run number `run` if its demo defines `frame`.
async function animate(run) {
    const {data: [animated, configured]} = await bpl.eval('3 2=•nc "frame" "options"');
    if (!animated) return;
    const {fps, ...options} = configured ? await bpl.eval('options') : {};
    driver ??= fetch(new URL('../driver.js', import.meta.url)).then(r => r.text()).then(source => (0, eval)(source));
    const draw = await driver;
    // A wait can outlast its run. These checks stop the animation before it touches a later run's session or output.
    if (run !== runs) return;
    const ctx = Object.assign(show(out, 'canvas'), {width: 500, height: 500}).getContext('2d');
    let start;
    for (let i = 0; ; i++) {
        let t;
        do t = await tick(); while (fps && t < (start ??= t) + i * 1000 / fps);
        if (run !== runs) return;
        let frame;
        try { frame = await bpl.eval(`frame ${i}`); }
        catch (error) { show(out, 'pre', error.display, 'error'); return; }
        if (frame === undefined) return;
        draw(ctx, frame, options);
    }
}

function choose() {
    const name = location.hash.slice(1) || links[0].hash.slice(1);
    for (const a of links) a.classList.toggle('active', a.hash === `#${name}`);
    demo = demos[name];
    about.textContent = demo.about;
    input.setValue(demo.code);
    go();
}

document.querySelector('#run').onclick = go;
document.querySelector('#reset').onclick = () => { input.setValue(demo.code); go(); };
addEventListener('hashchange', choose);
choose();
