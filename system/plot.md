

# `•plot` — Plots

`X •plot Y` returns a plot spec: a keyed vector holding the data `Y` and
the settings `X`. Its renderer displays the spec as an SVG chart. Plain
text on the left is shorthand for `mark`. [Plots](../plot.ipynb) shows
each mark and setting with examples.

The structure of `Y` chooses the series and axes:

- A vector plots its values against `0…n-1`.
- A keyed vector of numbers uses its keys as x labels.
- A matrix plots one series per row. Row keys name the series. Column
  keys label x.
- With the `cell` mark, each row of a matrix is a row of cells, coloured
  by value. Row keys label the rows, and row 0 is at the top.
- A table, a keyed vector of equal-length columns, plots each column as
  a series. A column named `x` supplies the x values.
- A vector or matrix of plots draws a figure.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Setting</th>
<th>Holds</th>
<th>Default</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>mark</code></td>
<td><code>"line"</code>, <code>"point"</code>, <code>"bar"</code> or
<code>"cell"</code></td>
<td><code>"line"</code></td>
</tr>
<tr>
<td><code>title</code></td>
<td>Chart title</td>
<td>none</td>
</tr>
<tr>
<td><code>width</code>, <code>height</code></td>
<td>Size in pixels</td>
<td><code>600</code>, <code>400</code></td>
</tr>
<tr>
<td><code>x</code>, <code>y</code></td>
<td><code>title</code>, <code>scale</code> (<code>"linear"</code> or
<code>"log"</code>), <code>ticks</code>, and <code>axis</code>
(<code>$f</code> hides that axis)</td>
<td></td>
</tr>
<tr>
<td><code>legend</code></td>
<td><code>position</code> (<code>"end"</code> or a corner) and
<code>border</code></td>
<td>none</td>
</tr>
<tr>
<td><code>grid</code></td>
<td><code>$f</code> hides the grid lines</td>
<td><code>$t</code>, or <code>$f</code> for cells</td>
</tr>
<tr>
<td><code>axes</code></td>
<td><code>$f</code> hides both axes, with their ticks and titles. An
<code>axis</code> setting in <code>x</code> or <code>y</code> overrides
it for that axis</td>
<td><code>$t</code></td>
</tr>
<tr>
<td><code>flip</code></td>
<td><code>$t</code> swaps the axes</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>palette</code></td>
<td>Colours for numbers: <code>"viridis"</code>, <code>"gray"</code>, or
a list of colours</td>
<td><code>"viridis"</code></td>
</tr>
<tr>
<td><code>colorbar</code></td>
<td><code>$t</code> shows the colour scale beside the plot</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>color</code>, <code>size</code>, <code>labels</code></td>
<td>Styles for every series</td>
<td></td>
</tr>
<tr>
<td><code>series</code></td>
<td>Styles for one series, keyed by its name</td>
<td></td>
</tr>
<tr>
<td><code>widths</code>, <code>heights</code>, <code>share</code></td>
<td>Figure cell sizes and shared axis ranges</td>
<td></td>
</tr>
</tbody>
</table>

`color` also takes one number per point. `palette` turns these numbers
into colours, over the range of every series in the plot. A cell takes
its colour from its own value unless `color` is set.

A direct [`•mime`](mime.qmd) call on a spec reports these errors:
`DOMAIN` for unknown settings or values, and for cells mixed with other
marks; `LENGTH` when series, colours, sizes or labels don’t match the x
values; `RANK` for data that isn’t a vector, matrix or table.
