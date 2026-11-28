

# `•explain` — Explain

`•explain text` runs `text` in the caller’s scope, as
[`⍎`](https://answerdotai.github.io/basedpl/glyphs/hydrant.html) does,
and returns a record. `value` is the result of `text`. Each other field
is a table of events that were recorded while `text` ran. A table is a
record of columns, as `•csv` returns, with one row for each event.

Events are recorded only while `•explain` runs. Events from code that
another thread runs at the same time are recorded in the same tables.
`•explain` computes a [pending](../terms.qmd#term-pending) result of
`text` before it returns, and adds no `forced` row for that.

[Performance](../performance.qmd) explains the behaviour that these
tables report. [`•memory`](memory.qmd) counts the live pending arrays,
and reports what a value keeps alive.

## Fused groups

### `fused`

One row for each [fused group](../terms.qmd#term-fused-group) that ran.
[Calls that wait](../performance.qmd#calls-that-wait) lists the calls
that can join a fused group.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>glyphs</code></td>
<td>the glyphs of the fused group’s calls, in the order they ran. A fold
that read the fused group’s result without storing it comes last</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items that the fused group processed</td>
</tr>
<tr>
<td><code>blocks</code></td>
<td>the number of blocks of items that the fused group processed, one
after another</td>
</tr>
<tr>
<td><code>runs</code></td>
<td>the number of pieces that the blocks were split into</td>
</tr>
<tr>
<td><code>copied</code></td>
<td>the number of items copied out of inputs before the calls read them,
as a reversed input needs</td>
</tr>
</tbody>
</table>

A block is split where it crosses the end of a long repeat or cycle in
an input, as when a row of 5,000 items is added to every row of a
matrix. When `runs` is greater than `blocks`, the fused group ran in
smaller pieces. When `copied` is 0, the fused group read every input in
place.

A fused group that ran a [compiled
loop](../terms.qmd#term-compiled-loop) also adds a `"compiled"` row to
`routes`. [Compiled loops](../performance.qmd#compiled-loops) gives the
sizes at which BPL compiles a fused group.

``` bpl
y←¿1000000ρ0
(•explain "+/ 2×y+1.5 - 1").fused            ⍝ ["glyphs":["+×-+/"] "items":[1000000]ₓ "blocks":[245]ₓ "runs":[245]ₓ "copied":[0]ₓ]
(•explain "2×⌽y+1").fused.copied             ⍝ [1000000]ₓ
t←y-0.25 ◇ (•explain "t×1-t").fused.glyphs    ⍝ ["--×"]
```

### `compiles`

One row for each piece of a fused group’s code that BPL compiled into
machine code.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>glyphs</code></td>
<td>the glyphs of the fused group’s calls</td>
</tr>
<tr>
<td><code>microseconds</code></td>
<td>the time that the compile took, which is about 250 for a fused group
of a few calls</td>
</tr>
</tbody>
</table>

### `stored`

One row for each pending array that a fused group read as an input after
the array was computed separately.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>glyphs</code></td>
<td>the glyph of the call that computed the array, or <code>""</code>
for a pending copy such as a reversal</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items in the array</td>
</tr>
<tr>
<td><code>reason</code></td>
<td>why the fused group did not compute the array as one of its own
calls</td>
</tr>
<tr>
<td><code>partway</code></td>
<td>always <code>$f</code></td>
</tr>
</tbody>
</table>

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th><code>reason</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"held outside the group"</code></td>
<td>The array is a pending copy that a name or another array also
holds</td>
</tr>
<tr>
<td><code>"kept by a fold"</code></td>
<td>An earlier fold read the array without storing its items, which left
the array pending</td>
</tr>
<tr>
<td><code>"a kind that the group doesn't hold"</code></td>
<td>The array’s items have a type that a fused group cannot process,
such as complex numbers</td>
</tr>
</tbody>
</table>

A pending array that is not a copy, and that a name or another array
holds, adds no row. The fused group computes it as one of its outputs
and stores its items in the same pass. [Names and
writes](../performance.qmd#names-and-writes) says when a dfn’s local
name stops holding its array.

``` bpl
y←¿1000000ρ0
q←1000005↑1⌽y ◇ (•explain "+/q+1").stored    ⍝ ["glyphs":[""] "items":[1000000]ₓ "reason":["held outside the group"] "partway":[$f]]
```

### `forced`

One row for each call or assignment that computed items of pending
arrays.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>function</code></td>
<td>the call’s function, or <a
href="https://answerdotai.github.io/basedpl/glyphs/left-arrow.html"><code>←</code></a>
for an assignment</td>
</tr>
<tr>
<td><code>line</code>, <code>column</code></td>
<td>its position in <code>text</code></td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items computed</td>
</tr>
<tr>
<td><code>reason</code>, <code>site</code></td>
<td>always empty</td>
</tr>
</tbody>
</table>

An assignment computes a pending array when it writes into the array,
displays it with `⎕←` or destructures it. A call that reads only a few
items computes only those items, as [Calls that
wait](../performance.qmd#calls-that-wait) describes.
`•prefs ["fusion":$t]` prints a line for each of these calls and
assignments as it happens.

``` bpl
y←¿1000000ρ0
e←•explain "≢⍕ 2×y+1.5 - 1"
e.forced.function                            ⍝ ["⍕"]
e.fused.glyphs                               ⍝ ["+×-"]
(•explain "5↑2×y+1").forced.items            ⍝ [5]ₓ
s←2×y+1 ◇ (•explain "5↑s").forced.items      ⍝ [5]ₓ
(•explain "5↑s").forced.items                ⍝ [1000000]ₓ
(•explain "5↑s").forced.items                ⍝ ⍬ₓ
r←⌽y ◇ (•explain "r₀←0").forced.function   ⍝ ["←"]
```

## Maps and items

### `fallbacks`

One row for each [map](../terms.qmd#term-map) that called its operand
once for each cell. The map did so because its [batched
call](../terms.qmd#term-batched-call) stopped, or because an earlier
batched call of the same dfn stopped and the map did not try one.
[Evaluation](../evaluation.qmd#functions-and-effects) describes batched
calls.

<table>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>reason</code></td>
<td>why the batched call stopped</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of cells</td>
</tr>
</tbody>
</table>

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th><code>reason</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"an effect"</code></td>
<td>The operand writes output, reads input, formats a cell’s value with
<a
href="https://answerdotai.github.io/basedpl/glyphs/thorn.html"><code>⍕</code></a>,
or calls a system function that reads or writes outside BPL or makes
something new on each call, such as <code>•nget</code> or
<code>•time</code>. A batched call would do each of these once for all
the cells</td>
</tr>
<tr>
<td><code>"a write"</code></td>
<td>The operand writes to a name outside the map, appends to a value
that differs from cell to cell, or writes into such a value in a way
that would leave it in <a href="../terms.qmd#term-mixed-storage">mixed
storage</a></td>
</tr>
<tr>
<td><code>"a cell's value"</code></td>
<td>The operand reads a cell’s value, for example as a count, a position
or a dfn predicate. Each cell then runs the body that its predicate
selects</td>
</tr>
<tr>
<td><code>"no batch rule"</code></td>
<td>The operand calls a function that has no rule for batches of these
arguments, such as <a
href="https://answerdotai.github.io/basedpl/glyphs/down-shoe.html"><code>∪</code></a>.
When a primitive stopped the call, its glyph follows, as in
<code>"no batch rule for ∪"</code></td>
</tr>
<tr>
<td><code>"too many items"</code></td>
<td>A call would repeat an array that every cell shares into far more
items than its arguments hold, and the array is too large for the map to
run on chunks of cells</td>
</tr>
<tr>
<td><code>"a caught error"</code></td>
<td>An error guard would catch an error</td>
</tr>
<tr>
<td><code>"an earlier stop: "</code> and one of the reasons above</td>
<td>The operand is a dfn whose batched call stopped on an earlier call,
for a cause that would recur. The map did not try a batched call</td>
</tr>
<tr>
<td><code>"an error"</code></td>
<td>The batched call failed. If the call on a cell also fails, that call
reports the error</td>
</tr>
<tr>
<td><code>"the result"</code></td>
<td>The operand’s result was not a batch of the cells’ results. A
function is one such result. A strand is a batch only when each of its
items holds one atom for each cell and the items’ kinds can be combined.
A strand that holds a vector for each cell is not a batch, and neither
is one that mixes numbers and characters</td>
</tr>
</tbody>
</table>

``` bpl
(•explain "{2(×⍣ω)1}¨ι5ₓ").fallbacks  ⍝ ["reason":["a cell's value"] "items":[5]ₓ]
```

### `itemwise`

One row for each route that handled an array’s items one at a time. Most
calls run code that handles a whole array.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>function</code></td>
<td>the innermost call that ran the route, the target of a selective
assignment, or <code>•explain</code> for a route outside both</td>
</tr>
<tr>
<td><code>line</code>, <code>column</code></td>
<td>its position in <code>text</code></td>
</tr>
<tr>
<td><code>route</code></td>
<td>the name of the route</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items that the route handled</td>
</tr>
</tbody>
</table>

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th><code>route</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"item pairs"</code></td>
<td>A pervasive function was called on each pair of items of mixed
storage or of a nested array</td>
</tr>
<tr>
<td><code>"batched items"</code></td>
<td>Code with no rule for batches read the items of a batch one at a
time. The batched call can still succeed</td>
</tr>
<tr>
<td><code>"∂ items"</code></td>
<td>Code with no rule for <a
href="../glyphs/partial.qmd"><code>∂</code></a> read, one at a time, the
items of a value that <a
href="https://answerdotai.github.io/basedpl/glyphs/partial.html"><code>∂</code></a>
is differentiating</td>
</tr>
<tr>
<td><code>"traced items"</code></td>
<td>Code that was building a <a
href="../terms.qmd#term-program">program</a> read traced items one at a
time</td>
</tr>
<tr>
<td><code>"accumulation"</code></td>
<td>A sum added items one at a time, as the <code>accumulations</code>
path <code>"item by item"</code> does</td>
</tr>
<tr>
<td><code>"item lanes"</code></td>
<td>A fold or scan called its function on each pair of items in turn,
because the function has no <a
href="../terms.qmd#term-kernel">kernel</a> for its arguments. A
primitive on mixed storage is one case, and a dfn that no batched call
covered is another</td>
</tr>
<tr>
<td><code>"number parts"</code></td>
<td>Monadic <a href="../glyphs/vee.qmd"><code>∨</code></a> or <a
href="../glyphs/wedge.qmd"><code>∧</code></a> took the parts of integers
or exact numbers one at a time. Floats and complex numbers take one
pass</td>
</tr>
<tr>
<td><code>"nested labels"</code>, <code>"label ids"</code></td>
<td>A selection from a nested array labelled or placed its items one at
a time</td>
</tr>
<tr>
<td><code>"key groups"</code></td>
<td><a href="../glyphs/quad-equal.qmd">Key</a> called its operand once
for each group, because no fast combination covered the call</td>
</tr>
<tr>
<td><code>"batch cells"</code></td>
<td>A batched call ran a system function, or a deal by <a
href="../glyphs/inverted-question.qmd"><code>¿</code></a>, once for each
cell in turn</td>
</tr>
<tr>
<td><code>"inverse scan"</code></td>
<td>The inverse of a scan called the function’s inverse on each pair of
neighbouring items in turn, because the function has no kernel</td>
</tr>
</tbody>
</table>

``` bpl
m←1ₓ 2.5 ◇ (•explain "m+1").itemwise  ⍝ ["function":["+"] "line":[1]ₓ "column":[2]ₓ "route":["item pairs"] "items":[2]ₓ]
```

### `mixed`

One row for each write that left a compact array in mixed storage. Under
dynamic kinds, assigning an exact integer into an array of floats does
so, by [the write rule](../numbers.qmd).

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>code</code></td>
<td>the target or call that wrote</td>
</tr>
<tr>
<td><code>line</code>, <code>column</code></td>
<td>its position in <code>text</code></td>
</tr>
<tr>
<td><code>target</code></td>
<td>the array’s storage before the write, as <a
href="storage.qmd"><code>•storage</code></a> names it</td>
</tr>
<tr>
<td><code>written</code></td>
<td>the storage of the values written</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items in the array</td>
</tr>
</tbody>
</table>

``` bpl
w←4ρ0.5 ◇ (•explain "(ι2ₓ)@[1 3] w").mixed  ⍝ ["code":["(ι2ₓ)@[1 3]"] "line":[1]ₓ "column":[1]ₓ "target":["float64"] "written":["integer"] "items":[4]ₓ]
```

### `accumulations`

One row for each sum that adds items at indexed positions. These sums
include the gradients of selections under [`∂`](../glyphs/partial.qmd),
and modified assignments such as `z.[p]+←1`.

<table>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>items</code></td>
<td>the number of items added</td>
</tr>
<tr>
<td><code>size</code></td>
<td>the number of items in the result</td>
</tr>
<tr>
<td><code>path</code></td>
<td>how the items were added</td>
</tr>
</tbody>
</table>

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th><code>path</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"in place"</code></td>
<td>The items are added into the array’s buffer. Only that array holds
the buffer, and the sum cannot fail for any item, as with floats</td>
</tr>
<tr>
<td><code>"on a copy"</code></td>
<td>The items are added into a copy of the array, because the sum can
fail or because another array shares the buffer. A sum of ordinary
integers can fail by overflow under dynamic kinds</td>
</tr>
<tr>
<td><code>"rounds"</code></td>
<td>Some positions repeat. Each call adds one item at each position that
still has items to add</td>
</tr>
<tr>
<td><code>"one call"</code></td>
<td>The positions are distinct, and one call adds every item</td>
</tr>
<tr>
<td><code>"item by item"</code></td>
<td>The function is called once for each target, as a dfn is</td>
</tr>
</tbody>
</table>

These sums add no row:

- a gradient that BPL computes as direct reads and sums, including one
  of windows with a step of 1
- a sum whose positions are one ascending run, such as the gradient of
  Take or Drop, which is added as a block
- the gradient of a
  [`⌈`](https://answerdotai.github.io/basedpl/glyphs/upstile.html) or
  [`⌊`](https://answerdotai.github.io/basedpl/glyphs/downstile.html)
  fold

``` bpl
x←¿1000ρ0 ◇ p←¿1000ρ1000
(•explain "{+/, 3↕ω (×¹) 1 2 1}∂ x").accumulations.items  ⍝ ⍬ₓ
(•explain "{+/ω.[p]}∂ x").accumulations            ⍝ ["items":[1000]ₓ "size":[1000]ₓ "path":["in place"]]
z←1000ρ0ₓ ◇ (•explain "z.[p]+←1ₓ").accumulations.path  ⍝ ["on a copy"]
```

## Shortcuts

### `fast`

One row for each [fast combination](../terms.qmd#term-fast-combination)
that BPL used, and for each fold of products that ran as a matrix
product.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>rule</code></td>
<td>the name of the combination: <code>"partial grade"</code>,
<code>"partial sort"</code>, <code>"ranks"</code>,
<code>"group counts"</code>, <code>"group folds"</code>,
<code>"determinant"</code>, <code>"permanent"</code>,
<code>"masked select"</code> or <code>"contraction"</code></td>
</tr>
<tr>
<td><code>function</code></td>
<td>the function that used it</td>
</tr>
</tbody>
</table>

When BPL recognises a combination from the form of the code, `function`
is the same text for every combination of that kind: `"↑⍋"` for every
selection, and `"f/⊢⌸"` for every group fold.

``` bpl
y←¿1000000ρ0
(•explain "(10↑)∘⍋ y").fast.rule             ⍝ ["partial grade"]
```

### `routes`

One row for each fast path that succeeded, with totals for all its
calls.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>route</code></td>
<td>the name of the fast path</td>
</tr>
<tr>
<td><code>calls</code></td>
<td>the number of calls that took it</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items that those calls handled, or 0 for a route that
counts no items</td>
</tr>
</tbody>
</table>

The routes include tracing (`"trace"`), fused groups (`"fusion"`),
batched calls (`"batching"`), compiled loops (`"compiled"`) and the
[fast combinations](../terms.qmd#term-fast-combination). They also
include these:

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th><code>route</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"vector kernel"</code></td>
<td>a pervasive call computed by its <a
href="../terms.qmd#term-kernel">kernel</a> on stored arrays</td>
</tr>
<tr>
<td><code>"mapped kernel"</code></td>
<td>a <code>"vector kernel"</code> call that read an argument through a
mapping, not from consecutive items: an extended scalar, a row repeated
down a matrix, or a pending copy such as <code>⌽y</code></td>
</tr>
<tr>
<td><code>"bulk catenate"</code></td>
<td>a <a
href="https://answerdotai.github.io/basedpl/glyphs/comma.html"><code>,</code></a>
or <a
href="https://answerdotai.github.io/basedpl/glyphs/comma-bar.html"><code>⍪</code></a>
fold over arrays of one rank and type, joined in one copy</td>
</tr>
<tr>
<td><code>"alternated fold"</code></td>
<td>a fold by <a
href="https://answerdotai.github.io/basedpl/glyphs/minus.html"><code>-</code></a>
or <a
href="https://answerdotai.github.io/basedpl/glyphs/obelus.html"><code>÷</code></a></td>
</tr>
<tr>
<td><code>"sliding fold"</code></td>
<td>a fold of windows</td>
</tr>
<tr>
<td><code>"mapped fold"</code></td>
<td>a fold that reads a pending copy without building the copy</td>
</tr>
<tr>
<td><code>"in place"</code></td>
<td>a mapped fold that reads its items where they are stored</td>
</tr>
<tr>
<td><code>"mapped product"</code></td>
<td>a matrix product that reads a pending copy larger than its result
without building the copy</td>
</tr>
</tbody>
</table>

`(•explain t).routes.route` lists the routes that `t` took.

### `declined`

One row for each optional shortcut that BPL considered and did not take,
for each reason. Another fast path can still handle a call that declined
one.

<table>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>route</code></td>
<td>the shortcut</td>
</tr>
<tr>
<td><code>reason</code></td>
<td>why BPL did not take it</td>
</tr>
<tr>
<td><code>calls</code></td>
<td>the number of calls that declined</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the number of items in those calls</td>
</tr>
</tbody>
</table>

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th><code>route</code></th>
<th><code>reason</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"fusion"</code></td>
<td><code>"a result kind that depends on values"</code></td>
<td>A call ran separately, because the type of its result depends on its
items, as with <a
href="https://answerdotai.github.io/basedpl/glyphs/radical.html"><code>√</code></a>
of floats under dynamic kinds</td>
</tr>
<tr>
<td><code>"fusion"</code></td>
<td><code>"an argument of another kind"</code></td>
<td>A call ran separately, because an argument is a stored array of
Booleans or integers whose type is not the type that the call computes
at, as in integers times a float. A fused group large enough to compile
on its first run takes such an argument</td>
</tr>
<tr>
<td><code>"fused fold"</code></td>
<td><code>"a copy that no call moves under"</code></td>
<td>A fold computed its argument, a pending copy, because the fold could
not read through the copy to its source</td>
</tr>
<tr>
<td><code>"fused fold"</code></td>
<td><code>"a fold that its group can't run"</code></td>
<td>A fold computed its argument, because the fused group cannot run
that fold</td>
</tr>
<tr>
<td><code>"compiled"</code></td>
<td>the name of a call, or
<code>"a kernel that doesn't compile"</code></td>
<td>A fused group that would have run a compiled loop ran its usual
kernels, because that kernel has no machine code</td>
</tr>
<tr>
<td>the name of a fast combination, such as
<code>"partial grade"</code></td>
<td><code>"arguments that don't suit it"</code></td>
<td>The general code ran</td>
</tr>
<tr>
<td><code>"batching"</code></td>
<td><code>"the batch pref is off"</code></td>
<td>A map did not try a batched call</td>
</tr>
<tr>
<td><code>"batching"</code></td>
<td><code>"a label level, or a ∂ call inside a batched run"</code></td>
<td>A map did not try a batched call, because it ran inside the
selection of an Under or a selective assignment, or inside a <a
href="https://answerdotai.github.io/basedpl/glyphs/partial.html"><code>∂</code></a>
call that a batched call was running</td>
</tr>
<tr>
<td><code>"batching"</code></td>
<td><code>"an argument too large to lay over the frame"</code>,
<code>"an argument that isn't compact"</code></td>
<td>A map did not try a batched call, because of that argument</td>
</tr>
<tr>
<td><code>"program"</code></td>
<td><code>"a call under another level"</code></td>
<td>A call did not use a program, because it ran inside a batched call
or another such transformation</td>
</tr>
<tr>
<td><code>"program"</code></td>
<td><code>"an argument that programs can't hold"</code></td>
<td>A call did not use a program, because a program cannot take one of
its arguments</td>
</tr>
<tr>
<td><code>"program"</code></td>
<td><code>"an effect"</code>, <code>"a write"</code>,
<code>"a random draw"</code>,
<code>"a free value that changes on every call"</code></td>
<td>An ordinary call’s trace stopped, for that reason</td>
</tr>
<tr>
<td><code>"program"</code></td>
<td><code>"a body outside dense elementwise reuse"</code></td>
<td>An ordinary call got no program, because its body does not give one
array, or has a step that cannot join a fused group</td>
</tr>
<tr>
<td><code>"in place"</code></td>
<td><code>"a repeated item"</code></td>
<td>A kernel copied an argument’s items into a buffer, because the
argument is one item used at every position</td>
</tr>
<tr>
<td><code>"in place"</code></td>
<td><code>"repeats of each item"</code></td>
<td>A kernel copied an argument’s items into a buffer, because each item
is used several times in a row, as a vector’s items are when the vector
is added to the rows of a matrix along the first axis</td>
</tr>
<tr>
<td><code>"in place"</code></td>
<td><code>"a cycle that wraps"</code></td>
<td>A kernel copied an argument’s items into a buffer, because the
argument’s items repeat as a cycle and the block being read passed the
end of the cycle</td>
</tr>
<tr>
<td><code>"in place"</code></td>
<td><code>"steps other than a forward run"</code></td>
<td>A kernel copied an argument’s items into a buffer, because the
argument is a pending copy whose items are not read in stored order, as
with a reversal</td>
</tr>
<tr>
<td><code>"contraction"</code></td>
<td>a reason such as
<code>"an operand that doesn't step along the fold"</code></td>
<td>A fold of elementwise products could not run as a matrix
product</td>
</tr>
</tbody>
</table>

A fold of a pending array that a name also holds stores the array’s
items in the same pass, and adds no row.

``` bpl
y←¿1000000ρ0
(•explain "+/⌽y").declined  ⍝ ["route":["fused fold"] "reason":["a copy that no call moves under"] "calls":[1]ₓ "items":[1000000]ₓ]
```

## Programs

### `programs`

One row for each call that built or replayed a
[program](../terms.qmd#term-program).
[Programs](../performance.qmd#programs) says which calls use programs,
and when BPL keeps one.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>event</code></td>
<td><code>"built"</code> for a program that this call recorded, or
<code>"replayed"</code> for one that BPL had kept</td>
</tr>
<tr>
<td><code>steps</code></td>
<td>the number of steps in the program</td>
</tr>
<tr>
<td><code>listing</code></td>
<td>the program as text, with one line for each value. A width suffix on
a value is listed as a <code>convert</code> step</td>
</tr>
</tbody>
</table>

A program for a [`∂`](../glyphs/partial.qmd) call holds the work of the
function and of its derivative. A program for an ordinary call holds the
work of that call’s body. The work that a program does when it runs is
reported in `fused`, `compiles` and `routes`.

When the trace of a
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) call
stops, `stopped` has a row in place of this one. When the trace of an
ordinary call stops, `declined` has a `"program"` row.

### `stopped`

One row for each reason that a [trace](../terms.qmd#term-trace) of a
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) call
stopped. The interpreter then ran the call without a program.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Holds</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>reason</code></td>
<td>why the trace stopped</td>
</tr>
<tr>
<td><code>calls</code></td>
<td>the number of calls, including calls that reused an earlier stopped
trace</td>
</tr>
<tr>
<td><code>items</code></td>
<td>the total number of items in the calls’ right arguments</td>
</tr>
<tr>
<td><code>site</code></td>
<td>always empty</td>
</tr>
</tbody>
</table>

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th><code>reason</code></th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"an effect"</code></td>
<td>The function writes output or has another effect</td>
</tr>
<tr>
<td><code>"a result whose type depends on values"</code></td>
<td>The function calls a function such as <a
href="https://answerdotai.github.io/basedpl/glyphs/radical.html"><code>√</code></a>,
whose result can be complex under dynamic kinds</td>
</tr>
<tr>
<td><code>"a free value that changes on every call"</code></td>
<td>The function uses an outside name or a bound array as positions, as
a count or in a predicate, and its value changed from call to call</td>
</tr>
<tr>
<td><code>"a function called on each item"</code></td>
<td>The function calls a function once for each item of a traced array.
A fold by a dfn does so, and so does an Each whose dfn has a
predicate</td>
</tr>
</tbody>
</table>

[What stops a trace](../autodiff.qmd#what-stops-a-trace) lists every
cause.
