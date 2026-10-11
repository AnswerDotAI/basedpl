

# System functions

A system function is a built-in function whose name starts with
[`•`](glyphs/bullet.qmd), as in `•ucs` or `•json`. Type `•` with Alt-y.
Names are case-insensitive. Many system functions take their options as
a left argument. A function named for a format reads that format, and
its [inverse](scripts.ipynb#superscripts) writes it: `•json` reads JSON,
and `•json⁻¹` writes it. Each name below links to its page.
`]help •name` shows the same page. Built-in values have names that start
with `$`, as [Constants and `$` names](system/constants.qmd) describes.

A function that takes one string, such as `•json` or `•element`, applies
to each string of an array of strings. A function that takes one number,
such as `•uuid`, applies to each number of an array. The results form an
array of the argument’s shape. An empty argument gives an empty array of
the function’s usual result, such as an empty list of strings from
`•src`. A left argument applies whole to each item. `•c`, `•date`,
`•path` and `•metadata` have their own rules for arrays, which their
pages give.

In Python, the workspace object `bpl` has each system function under its
name without `•`. Options become keyword arguments, as in
`bpl.json('{"a": null}', fill=0)`. `.undo` gives the inverse, as in
`bpl.json.undo(Y)` for `•json⁻¹`. [Python](python.ipynb) covers the
calling conventions.

## Text

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/c.qmd"><code>•c</code></a></td>
<td>Case</td>
</tr>
<tr>
<td><a href="system/ucs.qmd"><code>•ucs</code></a></td>
<td>Unicode</td>
</tr>
<tr>
<td><a href="system/normalize.qmd"><code>•normalize</code></a></td>
<td>Unicode normalization</td>
</tr>
<tr>
<td><a href="system/r.qmd"><code>•r</code></a></td>
<td>Regular expressions</td>
</tr>
</tbody>
</table>

## Data and files

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/csv.qmd"><code>•csv</code></a></td>
<td>CSV</td>
</tr>
<tr>
<td><a href="system/json.qmd"><code>•json</code></a></td>
<td>JSON</td>
</tr>
<tr>
<td><a href="system/literal.qmd"><code>•literal</code></a></td>
<td>Literals</td>
</tr>
<tr>
<td><a href="system/vfi.qmd"><code>•vfi</code></a></td>
<td>Numeric input</td>
</tr>
<tr>
<td><a href="system/date.qmd"><code>•date</code></a></td>
<td>Dates</td>
</tr>
<tr>
<td><a href="system/nget.qmd"><code>•nget</code></a></td>
<td>Read a file</td>
</tr>
<tr>
<td><a href="system/nput.qmd"><code>•nput</code></a></td>
<td>Write a file</td>
</tr>
<tr>
<td><a href="system/fetch.qmd"><code>•fetch</code></a></td>
<td>Fetch a URL</td>
</tr>
<tr>
<td><a href="system/path.qmd"><code>•path</code></a></td>
<td>Paths</td>
</tr>
<tr>
<td><a href="system/metadata.qmd"><code>•metadata</code></a></td>
<td>File metadata</td>
</tr>
<tr>
<td><a href="system/readdir.qmd"><code>•readdir</code></a></td>
<td>List a directory</td>
</tr>
<tr>
<td><a href="system/copy.qmd"><code>•copy</code></a></td>
<td>Copy</td>
</tr>
<tr>
<td><a href="system/rename.qmd"><code>•rename</code></a></td>
<td>Rename</td>
</tr>
<tr>
<td><a href="system/remove.qmd"><code>•remove</code></a></td>
<td>Remove</td>
</tr>
<tr>
<td><a href="system/mkdir.qmd"><code>•mkdir</code></a></td>
<td>Make a directory</td>
</tr>
<tr>
<td><a href="system/deflate.qmd"><code>•deflate</code></a></td>
<td>Compression</td>
</tr>
<tr>
<td><a href="system/hash.qmd"><code>•hash</code></a></td>
<td>Hashing</td>
</tr>
<tr>
<td><a href="system/uuid.qmd"><code>•uuid</code></a></td>
<td>UUIDs</td>
</tr>
</tbody>
</table>

## Display

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/element.qmd"><code>•element</code></a></td>
<td>XML elements</td>
</tr>
<tr>
<td><a href="system/xml.qmd"><code>•xml</code></a></td>
<td>XML text</td>
</tr>
<tr>
<td><a href="system/svg.qmd"><code>•svg</code></a></td>
<td>SVG pictures</td>
</tr>
<tr>
<td><a href="system/mime.qmd"><code>•mime</code></a></td>
<td>Rich display</td>
</tr>
<tr>
<td><a href="system/plot.qmd"><code>•plot</code></a></td>
<td>Plots</td>
</tr>
<tr>
<td><a href="system/image.qmd"><code>•image</code></a></td>
<td>Images</td>
</tr>
<tr>
<td><a href="system/canvas.qmd"><code>•canvas</code></a></td>
<td>Drawing with JavaScript</td>
</tr>
<tr>
<td><a href="system/prefs.qmd"><code>•prefs</code></a></td>
<td>Session settings</td>
</tr>
</tbody>
</table>

## Linear algebra

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/decompose.qmd"><code>•decompose</code></a></td>
<td>Matrix decompositions</td>
</tr>
</tbody>
</table>

## Random numbers and distributions

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/rand.qmd"><code>•rand</code></a></td>
<td>Generator</td>
</tr>
<tr>
<td><a
href="system/distribution.qmd"><code>•distribution</code></a></td>
<td>Distributions</td>
</tr>
</tbody>
</table>

## Programs

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/load.qmd"><code>•load</code></a></td>
<td>Load</td>
</tr>
<tr>
<td><a href="system/signal.qmd"><code>•signal</code></a></td>
<td>Signal</td>
</tr>
<tr>
<td><a href="system/storage.qmd"><code>•storage</code></a></td>
<td>Storage</td>
</tr>
<tr>
<td><a href="system/float.qmd"><code>•float</code></a></td>
<td>Float width</td>
</tr>
<tr>
<td><a href="system/time.qmd"><code>•time</code></a></td>
<td>Timing</td>
</tr>
<tr>
<td><a href="system/host.qmd"><code>•host</code></a></td>
<td>Host facts</td>
</tr>
<tr>
<td><a href="system/delay.qmd"><code>•delay</code></a></td>
<td>Delay</td>
</tr>
<tr>
<td><a href="system/js.qmd"><code>•js</code></a></td>
<td>JavaScript functions, in the browser</td>
</tr>
</tbody>
</table>

## Introspection

<table>
<thead>
<tr>
<th>Name</th>
<th>Description</th>
</tr>
</thead>
<tbody>
<tr>
<td><a href="system/nc.qmd"><code>•nc</code></a></td>
<td>Name class</td>
</tr>
<tr>
<td><a href="system/nl.qmd"><code>•nl</code></a></td>
<td>Name list</td>
</tr>
<tr>
<td><a href="system/src.qmd"><code>•src</code></a></td>
<td>Source</td>
</tr>
<tr>
<td><a href="system/ex.qmd"><code>•ex</code></a></td>
<td>Erase</td>
</tr>
</tbody>
</table>
