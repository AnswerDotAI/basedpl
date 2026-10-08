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
- `image/png` and `image/jpeg`: encoded images.

`run` returns only when the code finishes. To keep the page responsive, run BPL in a Web Worker. The playground's [`worker.js`](https://github.com/AnswerDotAI/basedpl/blob/main/nbs/playground/worker.js) and [`page.js`](https://github.com/AnswerDotAI/basedpl/blob/main/nbs/playground/page.js) show a complete page.

## Differences from native BPL

- `•readdir`, `•metadata`, `•copy`, `•rename`, `•remove`, `•mkdir`, `•path` and `•delay` don't exist. `•nput` writes only to `"-"`.
- File reads and `•fetch` are synchronous `XMLHttpRequest`s, relative to the URL given to `configure`.
- `•r` uses JavaScript's `RegExp`, with its syntax and replacement templates.
- `•date⁻¹` has no `locale` option.
- Function calls nest at most 410 deep. A deeper call is a `LIMIT` error.

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
