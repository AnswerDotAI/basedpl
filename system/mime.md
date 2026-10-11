

# `•mime` — Rich display

`•mime Y` returns the MIME bundle that display uses for `Y`. The bundle
is a keyed vector from MIME types to text, or to bytes for a binary type
such as `image/png`. It always has `text/plain`. If `Y` has a renderer,
`•mime` calls it with `Y` as `⍵` and adds its entries.

The built-in renderers add these types:

- `text/html`: element functions. Natively, a [`•canvas`](canvas.qmd)
  drawing too, as HTML that holds its data and the script that draws it.
- `image/svg+xml`: [`•svg`](svg.qmd) and [`•plot`](plot.qmd).
- `image/png`: [`•image`](image.qmd). An animation is an animated PNG in
  both builds.
- `image/x-rgba;width=N`: a still `•image` picture in the browser build,
  as RGBA pixels, `N` pixels wide. In JavaScript,
  `new ImageData(new Uint8ClampedArray(bytes.buffer), N)` makes a canvas
  image from the bytes.
- `application/x-bpl-canvas+json` and `application/x-bpl-canvas-data`: a
  `•canvas` drawing in the browser build. The npm package’s `canvas.js`
  is a script whose value is a function that draws it:
  `(0, eval)(script)(canvas, json, bytes)`, where `canvas` is a
  `<canvas>` element and `json` and `bytes` are the two entries.

`F •mime Y` returns `Y` with the renderer that `F` holds, as in
`{["text/html":"</b>",⍨"<b>",⍕⍵]}ᵘ •mime Y`. A MIME type on the left
displays `Y` itself as that type, as in `"text/markdown" •mime "*hi*"`.
An atom becomes a scalar, because only an array can hold a renderer.

A result keeps a renderer when it has an item for each item of its
argument, as pervasive functions, Each and scans do. Rearranging or
selecting from an array without removing an axis also keeps it, as
reversal, transpose, take, replicate and catenation do. Changes in place
keep it. Other functions drop it, as reductions and `⍴` do. When two
arguments have different renderers, the result has none. Match ignores
renderers.

Display shows the text form when a renderer fails. Only a direct `•mime`
call reports the error. [Rich display](../xml.ipynb#rich-display) covers
renderers.

Errors: `DOMAIN` for a left argument that holds neither a function nor
text.
