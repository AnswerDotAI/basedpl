

# `•prefs` — Session settings

`•prefs Y` applies the settings in the record `Y` to this session, and
returns every setting. `•prefs ⍬` changes nothing.

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
<td><code>box</code></td>
<td><code>$t</code> draws arrays in boxes</td>
<td><code>$t</code> in the REPL and the BPL Jupyter kernel,
<code>$f</code> elsewhere</td>
</tr>
<tr>
<td><code>trees</code></td>
<td><code>$t</code> shows functions and dissected expressions as trees;
<code>$f</code> uses linear notation. Independent of
<code>box</code></td>
<td>as <code>box</code></td>
</tr>
<tr>
<td><code>fns</code></td>
<td><code>$f</code> leaves output made inside functions unboxed</td>
<td>as <code>box</code></td>
</tr>
<tr>
<td><code>dissect</code></td>
<td><code>$t</code> shows the bound expression before each normal
result, using <code>trees</code></td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>limit</code></td>
<td>The most items display shows in full. <code>∞</code> shows every
item</td>
<td><code>1000</code></td>
</tr>
<tr>
<td><code>edges</code></td>
<td>Positions shown at each end of a long axis</td>
<td><code>3</code></td>
</tr>
<tr>
<td><code>prec</code></td>
<td>Significant digits shown for each float. <code>∞</code> shows every
digit</td>
<td><code>10</code></td>
</tr>
<tr>
<td><code>width</code></td>
<td>The widest line display shows. <code>∞</code> allows any width</td>
<td>The terminal’s width in the REPL, <code>∞</code> elsewhere</td>
</tr>
<tr>
<td><code>tolerance</code></td>
<td>The relative tolerance of approximate comparison of 64-bit floats,
from 0 to <code>1E¯14</code>. Narrower floats compare exactly</td>
<td><code>1E¯14</code></td>
</tr>
<tr>
<td><code>float</code></td>
<td>The width of new floats in bits, 16, 32 or 64</td>
<td><code>64</code></td>
</tr>
</tbody>
</table>

Display shows part of an array of more than `limit` items: the first and
last `edges` positions of each axis longer than twice `edges`. `…`
replaces the hidden columns, `⋮` the hidden rows, and `⋱` sits where
they cross. A nested array follows the same rule on its own. A line
wider than `width` hides columns in the same way, keeping as many at
each end as fit. `⎕←` always shows every item, and rounds floats to
`prec` as display does. [`⍕`](../glyphs/format.qmd) and source text
always hold every item and every digit.

`tolerance` sets how approximate numbers compare. `x` equals `y` when
`|x-y|` is at most the tolerance times the larger magnitude. The setting
applies to 64-bit floats. 32-bit and 16-bit floats always compare
exactly. Comparison at a tolerance of 0 is exact and faster. A float
still stands for a count, a position or a Boolean when it lies within
`1E¯14` of one, whatever the setting. [Power](../glyphs/power.qmd) `⍣∞`
stops at two results within `1E¯14` of each other, or within 4 units in
the last place of a 32-bit or 16-bit float, whatever the setting.

`float` sets the default width of floats, in bits: the width of bare
numbers in code parsed after the change, of numbers that `•csv`,
`•json`, `•vfi` and `•image` read, of random numbers, and of floats that
a function gives from exact arguments. Code already parsed keeps its
numbers. A change partway through some code therefore applies from the
next input, or from text that [`⍎`](../glyphs/execute.qmd) parses later.
Leading `•prefs` calls on a literal record are the exception. They run
before the rest of their code is parsed, as in the examples below. A
script, a notebook cell or a line typed at the REPL can set its own
width this way. In a module that [`•load`](load.qmd) runs, they run in
order with the rest of its code. See
[Numbers](../numbers.qmd#float-widths).

``` bpl
_←•prefs ["tolerance":0] ⋄ 0.3=0.1+0.2    ⍝ $f
_←•prefs ["tolerance":1E¯14] ⋄ 0.3=0.1+0.2    ⍝ $t
_←•prefs ["tolerance":1E¯14] ⋄ 1ₛ=1.0000001ₛ    ⍝ $f
•prefs ["float":32] ⋄ •storage 1.5    ⍝ "float32"
•prefs ["float":64] ⋄ •storage 1.5    ⍝ "float64"
```

Errors: `DOMAIN` for an unknown setting or an invalid value.
