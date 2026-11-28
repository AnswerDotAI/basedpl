

# `•canvas` — Drawing with JavaScript

`•canvas source` returns a drawing function made from `source`, the
source of a JavaScript function such as `(ctx, data, options) => …`. For
a drawing function `draw`, `options draw data` gives a record of the
source, the options and the data, which notebooks and the
[playground](../playground.qmd) display as a canvas. To display it, they
call the JavaScript function with the canvas’s 2D context, the data and
the options. If no options are given, the JavaScript function gets an
empty object as its options. The values are converted to JavaScript as
[`•js`](js.qmd) describes.

When there is no canvas, as in the REPL, the record is displayed as one
line of text: `canvas: N frames` when the options give `fps` and the
data is a vector of `N` frames, and `canvas` otherwise. Like every
option, `fps` goes to the JavaScript function, which decides what it
does. The canvas driver
[`driver.js`](https://github.com/AnswerDotAI/basedpl/blob/main/nbs/driver.js)
plays a vector of frames in a loop at `fps` frames per second.

BPL computes the data, and the JavaScript decides how to show it,
including the canvas’s size, which it sets through `ctx.canvas`:

``` bpl
bars←•canvas "(ctx, d, o) => { ctx.canvas.width = o.width; d.data.forEach((v, i) => ctx.fillRect(20 * i, 100 - v, 15, v)); }"
chart←["width":80] bars 30 80 55 100
chart.options.width   ⍝ 80
```

Errors: `DOMAIN` when the options or data hold a function.
