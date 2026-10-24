

# `•prefs` — Session settings

`•prefs Y` applies the settings in the record `Y` for the whole process,
and returns every setting except `seed`. The result is shy when `Y` sets
something, as an assignment’s is. `•prefs ⍬` changes nothing, and shows
every setting.

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
<td><code>$t</code> draws matrices, nested arrays and strings in boxes,
and leaves other vectors plain</td>
<td><code>$t</code> in the REPL, the BPL Jupyter kernel and IPython once
it loads BPL’s magics, <code>$f</code> elsewhere</td>
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
<td><code>fusion</code></td>
<td><code>$t</code> writes a line for each call that ends a fused
computation, as described below</td>
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
<td><code>notation</code></td>
<td>Generated function and operator notation: <code>"apl"</code>,
<code>"simplified"</code> or <code>"traditional"</code></td>
<td><code>"apl"</code></td>
</tr>
<tr>
<td><code>hanzi_aliases</code></td>
<td><code>$t</code> enables Hanzi glyph aliases. Other Hanzi remain name
characters. Requires a leading declaration on a literal record</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>mathish</code></td>
<td><code>$t</code> applies <a
href="../numbers.qmd#mathish-mode">Mathish mode</a> to <a
href="../terms.qmd#term-mathish-types">Mathish types</a> under dynamic
kinds</td>
<td><code>$t</code></td>
</tr>
<tr>
<td><code>float</code></td>
<td>The width of new floats in bits, 16, 32 or 64</td>
<td><code>64</code></td>
</tr>
<tr>
<td><code>kinds</code></td>
<td><code>"static"</code> makes each result’s kind follow from its
arguments’ kinds and these settings alone. <code>"dynamic"</code> lets
some functions choose it from values, as described below</td>
<td><code>"dynamic"</code></td>
</tr>
<tr>
<td><code>int</code></td>
<td>The width of integers in bits under static kinds, 32 or 64</td>
<td><code>64</code></td>
</tr>
<tr>
<td><code>threads</code></td>
<td>The number of threads that large calls split across. <code>0</code>
gives the default. <code>1</code> keeps every call on one thread</td>
<td><code>0</code></td>
</tr>
<tr>
<td><code>batch</code></td>
<td><code>$t</code> lets each <a href="../terms.qmd#term-map">map</a>
call its operand once on all its cells together, as described below</td>
<td><code>$t</code></td>
</tr>
<tr>
<td><code>seed</code></td>
<td>A nonnegative integer that seeds the session’s random numbers</td>
<td>A seed from the operating system</td>
</tr>
</tbody>
</table>

Display shows part of an array of more than `limit` items: the first and
last `edges` positions of each axis longer than twice `edges`. `…`
replaces the hidden columns, `⋮` the hidden rows, and `⋱` sits where
they cross. A nested array follows the same rule on its own. A line
wider than `width` hides columns in the same way, keeping as many at
each end as fit. `⎕←` always shows every item, and rounds floats to
`prec` as display does. [`⍕`](../glyphs/thorn.qmd) and source text
always hold every item and every digit.

`notation` changes generated function and operator displays, including
trees and dissected expressions. Dfn source, strings, record keys and
field separators stay unchanged. Generated source and structural APIs
keep canonical APL spellings.

`hanzi_aliases` controls accepted input independently of `notation`. Put
`•prefs ["hanzi_aliases":$t]` before any other code to enable Hanzi
glyph spellings. The setting persists across notebook cells and REPL
inputs. Loaded files use and update the same preferences as their
caller. See [Hanzi spellings](../syntax.qmd#hanzi-spellings).

Static calculations ignore `mathish`. `•prefs ⍬` still reports its
configured value.

`float` sets the default width of floats, in bits: the width of bare
numbers in code parsed after the change, of numbers that `•csv`,
`•json`, `•vfi` and `•image` read, of random numbers, and of floats that
a function gives from exact arguments. Code already parsed keeps its
numbers. A change partway through some code therefore applies from the
next input, or from text that [`⍎`](../glyphs/hydrant.qmd) parses later.
Leading `•prefs` calls on a literal record are the exception. They run
before the rest of their code is parsed, as in the examples below. A
script, a notebook cell, a line typed at the REPL, a module that
[`•load`](load.qmd) runs and text that `⍎` runs can each set its own
width this way. See [Numbers](../numbers.qmd#float-widths).

`kinds` sets whether a result’s kind can depend on its arguments’
values. Under the default, `"dynamic"`, `√¯4` gives `0ⱼ2`, `1j0` reads
as the float `1`, `⌊2.5` gives the exact integer `2`, `7ₓ÷2ₓ` gives
`7ᵣ2`, and exact integers grow without limit. Under `"static"`, each
result’s kind follows from the function, its arguments’ kinds and these
settings. Real arguments give real results, with NaN outside a
function’s domain. Complex results stay complex. `⌊`, `⌈` and monadic
`×` of floats give floats, and integer `÷` gives floats, where `⌊ₓ`,
`⌈ₓ`, `×ₓ` and `÷ₓ` give integers, converting as Rust’s `as` does.
Integers compute at the `int` width, or at a wider integer’s width, and
wrap where a result doesn’t fit. Exact fractions stay exact. An integer
beside a float takes the float’s width. An atom promotes as an array
does. Fused computations and a GPU need each result’s kind before they
run, and under static kinds more calls fuse, as the examples under
`fusion` show. A literal reads under the settings in force when its code
is parsed, as with `float`. See [Numbers](../numbers.qmd#static-kinds).

`threads` sets how many threads large calls split across. A call splits
only when it has enough work to repay the extra threads. Float results
can differ in their last bits between thread counts. The default, 0,
uses the number in the `RAYON_NUM_THREADS` environment variable where
it’s set, and one thread for each core otherwise. 1 runs every call on
the thread that evaluates the code. When several BPL processes run at
once, such as a pool of Python workers, set 1 in each of them to keep
them from competing for the cores. The browser build runs every call on
one thread, whatever the setting.

With `batch` on, a map calls its operand once on all its cells together
where it can, as [Evaluation](../evaluation.qmd#functions-and-effects)
describes. With `batch` set to `$f`, every map calls its operand once
for each cell. Results are the same either way.

`seed` restarts the session’s random numbers from the seed given, so the
same seed gives the same draws of [`¿`](../glyphs/inverted-question.qmd)
and of a distribution’s `sample`. The settings that `•prefs` returns
leave `seed` out, because the stream moves on with each draw. Restoring
saved settings therefore never reseeds it.

`fusion` shows where large computations stop waiting. Calls of these
glyphs on large arrays of floats run together, block by block, reading
each array once: monadic `+ - ÷ | * π ⊼ ⊽ ≤ ≥`, and dyadic
`+ - × ÷ | ⌈ ⌊ π` and the comparisons. A comparison runs with a later
call that reads it, as in `y×x>0`, unless a name also holds it.
Structural functions such as `⌽`, `⍉`, `↑`, `↓` and `↕` on large arrays
of any kind wait too, holding only where each item of their result comes
from in their argument. Each call waits until something reads its
result. A call that would make a chain of waiting calls hold more than
64 calls, or keep alive more than 8 times its largest array, first
computes the waiting results it reads. Fused calls, reductions, other
pervasive calls and `+.×` read a waiting structural result from its
argument, without building it. A read of a few items computes only those
items. Any other reader computes the whole result first, such as `⍋`,
display, and Python conversions other than NumPy views of structural
results. With `fusion` set to `$t`, the innermost call that computes
items of a waiting result writes a line naming its function and
position, such as `fusion ends at ⍋ (<input>:1:46)`. A reduction such as
`+/` of a waiting result runs with the calls and writes nothing.

[`•explain`](explain.qmd) records the same events. In the first example,
the comparison runs with the arithmetic and `+/`. In the second, monadic
`⌊` ends the fusion of `y×2`:

``` bpl
y←¿1000000⍴0
(•explain "+/(y+1)<|y-0.5").fused.glyphs    ⍝ ["-|+<+/"]
(•explain "+/⌊y×2").forced.function         ⍝ ["⌊"]
```

A call fuses when its result is floats of its arguments’ width, or
Booleans, and it can’t fail. This example checks the list above against
the monadic and dyadic forms of each numeric glyph. Halves of small
integers keep `∨` and `∧` quick. Under static kinds, every monadic glyph
in the list fuses except `○`, whose results are complex. There, the
kernels give NaN or floats where dynamic kinds would rerun a call on
complex numbers or give integers, so they can’t fail:

``` bpl
y←0.5×¿200000⍴100
fuses←{0<≢(•explain ⍵).fused.glyphs}
monads←"+-×÷|⌈⌊*⍟√!○π⊼⊽≤≥"
dyads←"+-×÷|⌈⌊*⍟√!π∨∧=≠<≤>≥"
({fuses "+/",⍵,"y"}¨monads)#monads      ⍝ "+-÷|*π⊼⊽≤≥"
({fuses "+/0.5",⍵,"y"}¨dyads)#dyads     ⍝ "+-×÷|⌈⌊π=≠<≤>≥"
•prefs ["kinds":"static"] ⋄ ({fuses "+/",⍵,"y"}¨monads)#monads    ⍝ "+-×÷|⌈⌊*⍟√!π⊼⊽≤≥"
•prefs ["kinds":"dynamic"]
```

``` bpl
•prefs ["mathish":$f] ⋄ 0.3=0.1+0.2    ⍝ $f
•prefs ["mathish":$t] ⋄ 0.3=0.1+0.2    ⍝ $t
1ₛ=1.0000001ₛ                        ⍝ $f
•prefs ["float":32] ⋄ (•storage 1.5).kind    ⍝ "float32"
•prefs ["float":64] ⋄ (•storage 1.5).kind    ⍝ "float64"
•prefs ["threads":1] ⋄ "threads"⊃•prefs ⍬    ⍝ 1ₓ
•prefs ["kinds":"static"] ⋄ √¯4    ⍝ $n
•prefs ["kinds":"static" "int":32] ⋄ 2147483647ₓ+1ₓ    ⍝ ¯2147483648ₓ
•prefs ["kinds":"dynamic" "int":64] ⋄ √¯4    ⍝ 0ⱼ2
•prefs ["seed":42] ⋄ a←¿6 6 6 ⋄ •prefs ["seed":42] ⋄ a≡¿6 6 6    ⍝ $t
```

Errors: `DOMAIN` for an unknown setting or an invalid value. Under
static kinds, `LIMIT` for an exact integer literal that doesn’t fit the
`int` width, or for a count or position that doesn’t fit it.
