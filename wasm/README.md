# basedpl

This package runs BasedPL (BPL) in a web page. BPL is an array language derived from APL, with ideas from J and BQN. The package holds the BPL interpreter compiled to WebAssembly, a language bar for typing BPL's glyphs, and a TextMate grammar for highlighting BPL code.

```js
import init, { Session, configure } from 'https://cdn.jsdelivr.net/npm/basedpl/basedpl_wasm.js';

await init();
configure(location.href, message => console.error(message));
new Session().run('avg←+/÷≢ ⋄ avg 2 4 9', output => console.log(output.data['text/plain']));   // logs 5
```

[BPL on the web](https://answerdotai.github.io/basedpl/web.html) documents this package: loading it, its outputs, values and JavaScript functions, drawing, the language bar, and how the gallery is built. The [playground](https://answerdotai.github.io/basedpl/playground.html) and [gallery](https://answerdotai.github.io/basedpl/gallery.html) run BPL in your browser. The [BPL documentation](https://answerdotai.github.io/basedpl/) covers the language.
