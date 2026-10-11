// Runs BPL in a Web Worker for a page. It loads the npm package's glue and module from the base URL that the page passes as the
// `pkg` parameter of this script's URL, and posts `ready` with the glyph rows. All messages share one session. A string is code to
// run: the worker posts each output as the program produces it, then a `reply` with the error or `null`. `{eval: code}` asks for the
// code's value: the worker posts it as a `value`, converted as `Session.eval` converts it, or a `reply` with the error. The value's
// typed arrays move to the page without a copy. Relative file reads resolve against this script's URL. A trap in the module leaves the
// session unusable. The worker then reports the trap, and the page starts a new worker.
const pkg = new URL(import.meta.url).searchParams.get('pkg');
const { default: init, Session, configure, symbols } = await import(`${pkg}basedpl_wasm.js`);

await init();
configure(location.href, message => postMessage({ type: 'panic', message }));
const session = new Session();
postMessage({ type: 'ready', symbols: JSON.parse(symbols()) });
// The buffers of the typed arrays in `value`. `postMessage` moves them to the page instead of copying them.
const buffers = value => ArrayBuffer.isView(value) ? [value.buffer]
    : value && typeof value === 'object' ? [...new Set(Object.values(value).flatMap(buffers))] : [];
onmessage = ({ data }) => {
    try {
        if (typeof data === 'string') postMessage({ type: 'reply', error: session.run(data, output => postMessage({ type: 'output', output })) });
        else {
            try {
                const value = session.eval(data.eval);
                postMessage({ type: 'value', value }, buffers(value));
            }
            catch (error) {
                if (error instanceof WebAssembly.RuntimeError) throw error;
                postMessage({ type: 'reply', error });
            }
        }
    } catch (e) { postMessage({ type: 'crash', message: String(e) }); }
};
