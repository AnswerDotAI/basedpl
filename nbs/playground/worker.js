// Runs BPL in a Web Worker for `page.js`. It loads the npm package's glue and module from the base URL that the page passes as the
// `pkg` parameter of this script's URL. It posts `ready` with the glyph rows, then runs each piece of code that the page sends in one
// session, posting each output as the program produces it. Relative file reads resolve against this script's URL. A trap in the
// module leaves the session unusable. The worker then reports the trap, and the page starts a new worker.
const pkg = new URL(import.meta.url).searchParams.get('pkg');
const { default: init, Session, configure, symbols } = await import(`${pkg}basedpl_wasm.js`);

await init();
configure(location.href, message => postMessage({ type: 'panic', message }));
const session = new Session();
postMessage({ type: 'ready', symbols: JSON.parse(symbols()) });
onmessage = ({ data: code }) => {
    try {
        const error = session.run(code, output => postMessage({ type: 'output', output }));
        postMessage({ type: 'reply', error });
    } catch (e) { postMessage({ type: 'crash', message: String(e) }); }
};
