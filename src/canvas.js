// Draws a `•canvas` output on `canvas`. `json` holds the drawing function's JavaScript source, its options and its data. Each typed
// array in the data is `{$typed, offset, length}`, a view of `bytes`. The native build runs this script from the output's HTML. A page
// that shows the browser build's outputs calls it with the output's two entries.
(canvas, json, bytes) => {
    const view = (_, v) => v?.$typed ? new globalThis[v.$typed](bytes.buffer, bytes.byteOffset + v.offset, v.length) : v;
    const { source, options, data } = JSON.parse(json, view);
    (0, eval)(source)(canvas.getContext('2d'), data, options);
}
