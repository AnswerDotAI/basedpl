

# `•js` — JavaScript functions

`•js source` returns a BPL function that calls the JavaScript function
`source` evaluates to. `f Y` calls it with `Y`, and `X f Y` with `X`
then `Y`. For example, after `h←•js "Math.hypot"`, `3 h 4` is `5`. Only
the browser build has `•js`. A page that runs BPL can also give it
JavaScript functions and values by name, with `Session.bind`.

Arguments cross into JavaScript, and results come back, as this table
shows. [`•canvas`](canvas.qmd), `Session.eval` and `Session.bind` follow
it too.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>BPL</th>
<th>JavaScript</th>
</tr>
</thead>
<tbody>
<tr>
<td>Number</td>
<td>Number. A Boolean is <code>true</code> or <code>false</code>.</td>
</tr>
<tr>
<td>String</td>
<td>String</td>
</tr>
<tr>
<td>Record</td>
<td>Object</td>
</tr>
<tr>
<td>Array of numbers</td>
<td><code>{shape, data}</code>. <code>data</code> is a
<code>Float32Array</code> for 32-bit floats and a
<code>Float64Array</code> for other approximate and fractional numbers.
For integers, it’s a <code>Uint8Array</code>, <code>Int16Array</code>,
<code>Int32Array</code> or <code>BigInt64Array</code>, at the width BPL
stores them. Complex numbers are their real and imaginary parts in turn,
with a last axis of length 2.</td>
</tr>
<tr>
<td>Any other array</td>
<td><code>{shape, data}</code>, with <code>data</code> an
<code>Array</code> of items</td>
</tr>
</tbody>
</table>

An array with axis names or keys also has `axis_names` or `axis_keys`. A
function or operator can’t cross into JavaScript. Coming back, a
JavaScript number becomes an approximate number, and a `BigInt` an exact
one. An `Array` becomes a vector. An object with `shape` and `data`
becomes an array, and any other object a record. A JavaScript function
becomes a BPL function. `undefined` and `null` give no result.

Errors: `DOMAIN` for source that doesn’t give a function, an exception
thrown by the JavaScript, or a value that can’t cross.
