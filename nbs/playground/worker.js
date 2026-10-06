// Runs BPL in a Web Worker for `page.js`. It loads the latest release of the npm package `basedpl` from jsDelivr, posts `ready` with the
// package's URL and the glyph rows, then runs each piece of code that the page sends in one session, posting each output as the program
// produces it. Relative file reads resolve against this script's URL. A trap in the module leaves the session unusable. The worker then
// reports the trap, and the page starts a new worker.
const pkg = 'https://cdn.jsdelivr.net/npm/basedpl/';
const { default: init, Session, configure, symbols } = await import(`${pkg}basedpl_wasm.js`);

await init();
configure(location.href, message => postMessage({ type: 'panic', message }));
const session = new Session();
postMessage({ type: 'ready', pkg, symbols: JSON.parse(symbols()) });
onmessage = ({ data: code }) => {
    try {
        const reply = session.request(JSON.stringify({ code }), output => postMessage({ type: 'output', output: JSON.parse(output) }));
        postMessage({ type: 'reply', reply: JSON.parse(reply) });
    } catch (e) { postMessage({ type: 'crash', message: String(e) }); }
};
