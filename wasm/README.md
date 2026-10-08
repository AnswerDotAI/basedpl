# basedpl

This package runs BasedPL (BPL) in a web page. BPL is an array language derived from APL, with ideas from J and BQN. The package holds the BPL interpreter compiled to WebAssembly, a language bar for typing BPL's glyphs, and a TextMate grammar for highlighting BPL code. The [BPL documentation](https://answerdotai.github.io/basedpl/) covers the language. The [playground](https://answerdotai.github.io/basedpl/playground.html) runs BPL in the browser with this package.

## Load the package

jsDelivr serves every file in the package. An unversioned URL, such as `https://cdn.jsdelivr.net/npm/basedpl/basedpl_wasm.js`, gives the latest release. To keep one release, put its version in the URL: `https://cdn.jsdelivr.net/npm/basedpl@<version>/basedpl_wasm.js`. To install the package instead, run `npm install basedpl`.

## Run BPL

```js
import init, { Session, configure } from 'https://cdn.jsdelivr.net/npm/basedpl/basedpl_wasm.js';

await init();
configure(location.href, message => console.error(message));
const session = new Session();
const error = session.run('avg←+/÷≢ ⋄ avg 2 4 9', output => console.log(output.data['text/plain']));
```

This logs `5`, and `error` is `null`.

- `init()` fetches the module from beside the glue.
- `configure(base, panic)` sets the URL that relative file reads resolve against, and the function that receives a panic's message. After a panic, the module can't run more code.
- `session.run(code, output)` runs `code`, then returns `null` or the error. The error is an object with `kind`, `message` and `display`. `display` holds the error as BPL shows it. A session keeps its names from one `run` to the next.
- `output` receives each output as the program produces it, as `{kind, data}`. `kind` is `"display"` for a displayed value, `"explicit"` for a value assigned to `⎕`, or `"text"` for text written with `"-" •nput`. Display and explicit outputs each end a line. Text output doesn't.

`data` maps MIME types to text, or to a `Uint8Array` for a binary type:

- `text/plain`: the output as text.
- `text/html`: markup from element functions.
- `image/svg+xml`: markup from `•svg` and `•plot`.
- `image/x-rgba;width=N`: RGBA pixels from `•image`, `N` pixels wide. `new ImageData(new Uint8ClampedArray(bytes.buffer), N)` makes a canvas image from them.
- `image/png` and `image/jpeg`: encoded images. An animation from `•image` is an animated PNG.
- `application/x-bpl-canvas+json` and `application/x-bpl-canvas-data`: a canvas from `•canvas`. `canvas.js` is a script whose value is a function that draws it: `(0, eval)(script)(canvas, data['application/x-bpl-canvas+json'], data['application/x-bpl-canvas-data'])`, where `canvas` is a `<canvas>` element.

`run` returns only when the code finishes. To keep the page responsive, run BPL in a Web Worker. The docs' [`worker.js`](https://github.com/AnswerDotAI/basedpl/blob/main/nbs/playground/worker.js) runs BPL in a worker, and [`bpl.js`](https://github.com/AnswerDotAI/basedpl/blob/main/nbs/playground/bpl.js) starts the worker and shows each output on the page.

## Values and JavaScript functions

`session.eval(code)` returns the value of `code` as a JavaScript value, or `undefined` when the code gives no value. A BPL error throws the error object. `session.bind(names)` binds each property of the object `names` as a BPL name. A JavaScript function becomes a BPL function that calls it, with the right argument, or the left then the right. Inside BPL, `•js source` makes such a function from JavaScript source. Values cross as the [`•js`](https://answerdotai.github.io/basedpl/system/js.html) page describes: a number is a number, a string a string and a record an object. Any other array is `{shape, data}`, with `data` a typed array when every item is a number.

```js
session.bind({ scale: 2, log: x => console.log(x) });
session.eval('scale×1 2 3');   // {shape: [3], data: Float64Array [2, 4, 6]}
session.run('log "hi"', () => {});   // logs "hi"
```

## Differences from native BPL

- `•js` exists only here.
- `•readdir`, `•metadata`, `•copy`, `•rename`, `•remove`, `•mkdir`, `•path` and `•delay` don't exist. `•nput` writes only to `"-"`.
- File reads and `•fetch` are synchronous `XMLHttpRequest`s, relative to the URL given to `configure`.
- `•r` uses JavaScript's `RegExp`, with its syntax and replacement templates.
- `•date⁻¹` has no `locale` option.
- Function calls nest at most 380 deep. A deeper call is a `LIMIT` error.

## Language bar

`lb.js` adds a bar of BPL's glyphs to the page. Clicking a glyph types it into the editor used last. In a textarea marked with `data-bpl`, or a Monaco editor whose language is `bpl`, a backtick followed by a glyph's name also types the glyph, as in the BPL REPL. `lb.js` and `input.js` are scripts whose value is a function. The bar needs the glyph rows from `symbols()` and the key layout from `layout.json`:

```js
import init, { symbols } from 'https://cdn.jsdelivr.net/npm/basedpl/basedpl_wasm.js';

const text = name => fetch(`https://cdn.jsdelivr.net/npm/basedpl/${name}`).then(r => r.text());
const [lb, input, layout] = await Promise.all(['lb.js', 'input.js', 'layout.json'].map(text));
await init();
(0, eval)(lb)(JSON.parse(symbols()), (0, eval)(input), JSON.parse(layout));
```

## Highlighting

`bpl.tmLanguage.json` is a TextMate grammar for BPL, with the scope name `source.bpl`. Shiki and editors that read TextMate grammars use it to highlight BPL code.
