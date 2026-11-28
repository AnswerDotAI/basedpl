

# Evaluation

After [Syntax](syntax.qmd) divides a statement into runs, BPL binds the
runs as it evaluates them. Binding depends on values. A name can hold a
subject, a function or an operator. A name’s case doesn’t decide which,
as it does in BQN. BPL looks a name up when the statement runs.

## Binding

Evaluation goes from right to left. Functions have no precedence over
each other. A function’s right argument is everything to its right. Its
left argument is the subject directly to its left, if there is one. The
value to a function’s left therefore decides whether the function gets
one argument or two. In `g⁝f4` with `g←÷`, the function `g` stands to
the left of `f`, which makes `f` monadic.

``` bpl
2×3+4                ⍝ 14
10-3-2               ⍝ 9
f←- ◇ 5 f 3          ⍝ 2
f←- ◇ g←÷ ◇ g⁝f4    ⍝ ¯0.25
```

Subjects side by side form a [strand](terms.qmd#term-strand), a vector
with one item for each subject. Each run is evaluated before the strand
forms. A literal strand counts as one item.

``` bpl
a←1 2 ◇ b←3 4 ◇ a b        ⍝ [[1 2] [3 4]]
a←1 ◇ b←2 ◇ a b+1          ⍝ 1 3
x←5 ◇ x 1 2                ⍝ [5 [1 2]]
```

A call’s result, a field read from a call’s result, a subscript’s
selection and a scope’s result all bind as subjects, whatever they hold.
A function that a call returns therefore can’t take arguments or join a
train where the call stands: in `+/÷(f 0)` with `f←{≢}`,
[`÷`](https://answerdotai.github.io/basedpl/glyphs/obelus.html) gets
[`≢`](https://answerdotai.github.io/basedpl/glyphs/equal-underbar-slash.html)
as its argument, which raises a `DOMAIN` error. Name a returned function
before you call it, and call a function selected from an array with
[Agenda](glyphs/diamond-underbar.qmd). A dot path names a field, so
`T.f 3` calls the function in field `f`, whereas the scope `T.(f)` gives
that function as a subject.

``` bpl
f←{≢} ◇ g←f 0 ◇ +/÷g 1 2 3        ⍝ 2
f←{≢} ◇ (f 0) 1 2 3               ⍝ [≢ [1 2 3]]
fs←[- ×] ◇ fs₁ 3                  ⍝ [× 3]
fs←[- ×] ◇ 2 (1⍚fs) 3             ⍝ 6
T←["f":-] ◇ (T.f 3) (T.(f) 3)     ⍝ [¯3 [- 3]]
```

An array that holds a function as an item binds as a function only where
a subject follows it. There it calls each function item on the whole
arguments, and the result for any other item is that item, as [Functions
in arrays](arrays.ipynb#functions-in-arrays) shows. Followed by a
function, or by nothing, it’s a subject: a left argument, a strand item
or an argument. `f fs x` applies `f` to `fs x`, and `fs f x` gives `f`
the left argument `fs`.

``` bpl
fs←[+ × ÷] ◇ 2 fs 8          ⍝ 10 16 0.25
fs←[+ × ÷] ◇ ≢ fs 4          ⍝ 3ₓ
fs←[+ × ÷] ◇ fs≡fs           ⍝ $t
fs←[+ × ÷] ◇ ≢fs             ⍝ 3ₓ
fs←[+ × ÷] ◇ 1 fs            ⍝ [1 [+ × ÷]]
```

## Operators and operands

Operators bind before functions apply. An operator’s left
[operand](terms.qmd#term-operand) is the whole function to its left. Its
right operand is the one [token](terms.qmd#term-token) to its right. A
literal strand counts as one token: in `f⍤1 2 M`, the rank is `1 2`. Put
brackets around a vector argument after a literal operand, as in
`f⍤1[2 3]`. An argument that touches the operand is not part of the
operand: `+/⍤1m` applies `+/⍤1` to `m`, and `'a',⍣3'b'` applies `'a',⍣3`
to `'b'`.

[`⁝`](https://answerdotai.github.io/basedpl/glyphs/tricolon.html)
separates a numeric operand from a numeric argument. `f⍣¯25` has operand
`¯25`, and `f⍣¯2 5` has operand `¯2 5`. `f⍣¯2⁝5` has operand `¯2` and
argument `5`. With both a left and a right argument, parentheses around
the derived function show its boundary. [Composing
functions](composition.qmd#separating-an-operand-from-its-argument)
gives the forms and where each reads well.

Each run counts as one token, as a parenthesised group does. An operator
at the end of a run takes the next run as its right operand, as it takes
a touching token: in `⌊@ 10× x`, the right operand of
[`@`](https://answerdotai.github.io/basedpl/glyphs/at.html) is the run
`10×`. An operator at the start of a run takes the previous run as its
left operand: in `≢⊢ ⌸x`, the left operand of
[`⌸`](https://answerdotai.github.io/basedpl/glyphs/quad-equal.html) is
the run `≢⊢`. These rules also apply to names that hold operators.

A space can therefore group a short function operand: `⊂⊢ ⌸` is `(⊂⊢)⌸`.
With two arguments, `0 1 0 ⊂⊢ ⌸ "abc"` keeps the operand and each
argument in its own run. [Operators on
trains](composition.qmd#operators-on-trains) gives examples.

``` bpl
m←2 3ρι6ₓ ◇ +/⍤1 m               ⍝ [3 12]ₓ
m←2 3ρι6ₓ ◇ +/⍤1m                ⍝ [3 12]ₓ
⌊@ 10× 1.25 2.78                 ⍝ 1.2 2.7
x←0 1 1 0 1 1 ◇ ≢⊢ ⌸x            ⍝ [2 4]ₓ
x←5 ◇ ≥⍣3x                      ⍝ 8
≥⍣3⁝5                           ⍝ 8
```

When an operator takes only a function operand, an array operand that
holds a function as an item calls each function item on the whole
arguments. The result for any other item is that item. The results are
assembled as Rank assembles its results, with the array’s shape as their
frame. An array with no function items acts as a constant function, like
`A⍨`. An operator that takes either a function or a subject as an
operand treats every array as a subject. Reduce and scan need a
function, or an array that holds one. `5/` raises a `DOMAIN` error, and
Replicate is written
[`#`](https://answerdotai.github.io/basedpl/glyphs/hash.html).

``` bpl
5¨ 1 2 3                         ⍝ 5 5 5
[+ -]¨ 1 2                       ⍝ [1 ¯1;2 ¯2]
```

## Trains

A run, group or statement that ends in a function is a
[train](terms.qmd#term-train). BPL builds a train in two steps:

1.  Each subject [binds](terms.qmd#term-bind) the one
    [part](terms.qmd#term-part) after it. `2×` is `2↣×`, and the parts
    of `2|#⊢` are `2|`,
    [`#`](https://answerdotai.github.io/basedpl/glyphs/hash.html) and
    [`⊢`](https://answerdotai.github.io/basedpl/glyphs/right-tack.html).
    To bind a whole train, put it in parentheses, as in `2(+×-)`.
2.  BPL builds the train from its last part leftwards, as APL does. A
    bound part takes one argument, so it forms an
    [atop](terms.qmd#term-atop) with the train built so far. Any other
    part forms a [fork](terms.qmd#term-fork) with the part before it and
    the train built so far, or an atop when it comes first.

A fork `f g h` gives `(f ω) g (h ω)`, and an atop `f g` gives `f (g ω)`.
So `2|#⊢` is the fork `(2|)#⊢`, and `0.5×⊢+÷` is the atop
`(0.5×)∘(⊢+÷)`, as in APL. With two arguments, a fork gives
`(α f ω) g (α h ω)`, and an atop gives `f (α g ω)`. A bound function
takes one argument only: `3 (2×) 4` raises a `SYNTAX` error.

``` bpl
f←32+1.8× ◇ f100     ⍝ 212
+/÷≢ 1 2 3 6       ⍝ 3
0.5×⊢+÷ 2          ⍝ 1.25
2|#⊢ 5 6 7         ⍝ 5 7
2 (1-×) 5            ⍝ ¯9
```

A space before an argument keeps the train separate from it. `+/÷≢ x`
applies the train `+/÷≢` to `x`. `+/÷≢x` is one run, which reads as
`+/(÷(≢x))`. A train whose subjects and functions alternate, such as
`32+1.8×`, gives the same result either way.

[`⁝`](https://answerdotai.github.io/basedpl/glyphs/tricolon.html) keeps
adjacent names distinct without separating their run: `1+⌽f⁝x` applies
`f` to `x`. A space there changes the grouping: `1+⌽f x` applies the
train `1+⌽f` to `x`. A literal can touch a name without a separator, as
it touches a glyph: `1+⌽f5`. A run that starts with a function is a
call: `×2` is the sign of 2.

A [fast combination](terms.qmd#term-fast-combination) is a combination
of functions that BPL computes with a dedicated algorithm. `10↑⍋v`
selects the first 10 positions of the grade without ordering the rest. A
combination can also be more accurate than its functions computed in
turn: `⊛1+x` keeps the digits of a small `x` that `1+x` rounds away. BPL
recognises a combination whether it is written as an expression, a
train, a composition or an operator’s operand, provided its functions
and constants are glyphs and literals. `10↑⍋v`, `10↑⍋ v` and `(10↑)∘⍋ v`
use the same algorithm from their first call. BPL does not recognise the
combination in `one←1 ◇ ⊛one+x` or `ln←⊛ ◇ ln 1+x`, where a name stands
for the constant or the function. Each computes `1+x` and then
[`⊛`](https://answerdotai.github.io/basedpl/glyphs/circle-star.html).
Results can differ between spellings, as the
[Accuracy](numbers.qmd#accuracy) section of Numbers describes. The glyph
pages list each combination under “Fast combinations”.
[`•explain`](system/explain.qmd) reports each selection, rank, product
and Key combination that a run uses. [Pending](terms.qmd#term-pending)
results give other fast paths: when the windows of `50↕v` are pending,
`+/50↕v` folds them by sliding over `v`, without building them.

## Agreement and pervasion

[Pervasive functions](terms.qmd#term-pervasive-function) align the
leading axes of their arguments. Missing trailing axes count as
length 1. Axes of equal length agree. An axis of length 1 expands to the
other length, including to zero. Axes of length 1 in both arguments
remain. Any other mismatch raises a `LENGTH` error.

``` bpl
[1 2 3◇4 5 6]+10 20 ⍝ [11 12 13◇24 25 26]
[[10]◇[20]]+[1 2 3◇] ⍝ [11 12 13◇21 22 23]
ρ[[10]◇]+1 2 3       ⍝ [3 1]ₓ
```

Pervasion repeats these rules inside nested items. Each and Rank frames
also use leading agreement. Products, replication, indexing and
assignment have their own rules. BPL’s agreement extends APL’s. NumPy
aligns trailing axes instead.

## Axes and indices

Positions and axes count from 0. Negative positions and axes count from
the end. A count or position must be a whole number. With Mathish on, a
float within `1E¯14` of a whole number also counts as a whole number.

[Index](glyphs/squad.qmd)
[`⌷`](https://answerdotai.github.io/basedpl/glyphs/squad.html) selects
along leading axes. Its result has the shape of the positions, followed
by the shape of each selected cell.
[`:`](https://answerdotai.github.io/basedpl/glyphs/colon.html) selects
one whole axis;
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html) keeps
the remaining axes. [Dot indexing](glyphs/dot.qmd) writes the same
selection after an array: `m.[: 1]` is `[: 1]⌷m`. A
[subscript](scripts.ipynb#subscripts) selects one position on the
leading axis.

``` bpl
v←10 20 30 40 ◇ [[2 0]]⌷v       ⍝ 30 10
v←10 20 30 40 ◇ [⊂1]⌷v          ⍝ ⊂20
v←10 20 30 40 ◇ [[1]]⌷v         ⍝ [20]
v←10 20 30 40 ◇ v₋₁             ⍝ 40
m←[1 2 3◇4 5 6] ◇ 1⌷m         ⍝ 4 5 6
m←[1 2 3◇4 5 6] ◇ [: 1]⌷m       ⍝ 2 5
```

[Axis](glyphs/quad-colon.qmd)
[`⍠`](https://answerdotai.github.io/basedpl/glyphs/quad-colon.html)
applies a function along axes given as numbers or names, as in `+/⍠0`
and `+/⍠"month"`. [Rank](glyphs/jot-diaeresis.qmd)
[`⍤`](https://answerdotai.github.io/basedpl/glyphs/jot-diaeresis.html)
applies a function to trailing cells. `⍤0` passes an atom item as
itself, and an array item as a scalar holding it.

Some functions have a default axis:

- `/ \ ⌽ ,` work along the last axis.
- `⌿ ⍀ ⍪` work along the first axis; `⌽₀` reverses or rotates it.
- Replicate `N#Y` and the partitions `N⊂Y` and `N⊆Y` work along the
  first axis.
- Encode
  [`⊤`](https://answerdotai.github.io/basedpl/glyphs/down-tack.html)
  puts its digits on the last axis. Decode
  [`⊥`](https://answerdotai.github.io/basedpl/glyphs/up-tack.html) reads
  them from the last axis.

[`,`](https://answerdotai.github.io/basedpl/glyphs/comma.html),
[`⊂`](https://answerdotai.github.io/basedpl/glyphs/left-shoe.html) and
[`↑`](https://answerdotai.github.io/basedpl/glyphs/up-arrow.html) also
accept lists of axes.

``` bpl
+/⍠0 [1 2 3◇4 5 6]           ⍝ 5 7 9
```

## Equality and ordering

Comparison between exact numbers is exact. Floating comparison follows
[Mathish mode](numbers.qmd#mathish-mode). Infinity equals itself.
Infinity never equals a finite number.

``` bpl
0.3=0.1+0.2          ⍝ $t
1r3=1ᵣ÷3ᵣ            ⍝ $t
```

Search functions compare major cells.
[`ι`](https://answerdotai.github.io/basedpl/glyphs/iota.html) and
[`⍸`](https://answerdotai.github.io/basedpl/glyphs/iota-underbar.html)
search their left argument. `∊ ~ ∩` search their right argument. The
other argument’s cells have the same rank as the major cells of the
searched argument. A unit can’t be searched, because it has no major
cells: `s~" "` removes spaces, but `s~' '` raises a `RANK` error.

Search, membership, match and the functions that find distinct major
cells use the same comparison rule. Tolerance isn’t transitive. A search
finds the first match of each cell it looks for. Monadic `∪ ≠ = ∩ ⍷ #`
and Key treat real numbers as one value when, in ascending order, each
matches the next. The first and last of these numbers needn’t match. A
major cell that is an array counts as a copy of the first earlier
distinct cell that it matches.

``` bpl
≠[1 1+8E¯15 1+16E¯15]   ⍝ $t $f $f
[1]ι1+16E¯15            ⍝ 1ₓ
```

Grade and interval index ignore tolerance. They order values like this:

- Numbers come before characters, and characters before nested arrays.
- Numbers compare by value. NaN comes after every other number. Complex
  numbers compare by real part, then by imaginary part.
- Characters compare by code point.
- Nested arrays compare by rank, then by ravel lexicographically, then
  by shape. Prototypes don’t affect the ordering.

Grade is stable. Pervasive ordering (`< ≤ > ≥ ⌊ ⌈`) takes real numbers
and characters, and orders a number before a character, as grade does.

## Functions and effects

Dfns use lexical scope. Plain assignment is local. Modified and
selective updates change the nearest binding. Arrays have value
semantics: updating one name leaves other copies unchanged.

``` bpl
a←1 2 ◇ b←a ◇ a₀←9 ◇ b ⍝ 1 2
```

Every statement in a dfn body runs. The body’s last statement gives the
result. A final assignment returns its value without displaying it. An
assignment at top level displays nothing either. [Dfns](dfns.ipynb)
covers bodies, predicates and arguments.

The [tines](terms.qmd#term-tine) of a fork are also evaluated from right
to left. Items inside brackets are evaluated from left to right, as
statements are.

A [map](terms.qmd#term-map) with no cells calls its operand on
[prototypes](terms.qmd#term-prototype). An inner product `x f.g y` with
an empty argument calls `g` on prototypes, and also calls `f` when the
result is empty. Empty scan makes no calls. Generic reduction associates
to the right. Algebraic reductions such as `+/` may reassociate, as
[Accuracy](numbers.qmd#accuracy) describes. All scans accumulate left to
right.

When it can, a map calls its operand once on all its cells together, in
one [batched call](terms.qmd#term-batched-call). Each primitive that the
operand calls then runs once on the whole arguments: `{ω×ω+1}¨v` costs
about what `v×v+1` costs. A system function acts when it reads or writes
files, the network, the clock or the terminal, or makes something new on
each call, as `•nget`, `•time` and `•uuid` do. A system function that
doesn’t act, such as `•json` or `•ucs`, runs once for each cell within
the batched call. Batching preserves the result structure, errors,
output and writes to names. Floating results follow
[Accuracy](numbers.qmd#accuracy), without bit-for-bit agreement between
routes. [Seeded random draws](system/rand.qmd) can differ between
batched and per-cell execution.

A predicate that reads a cell’s value stops batching. The map then calls
its operand separately for each cell, running only the selected body.
Predicates that depend only on shared constants or cell shapes can stay
batched. For conditional work on whole arrays, use
[`⍚`](glyphs/diamond-underbar.qmd) or Under instead of a predicate.

When a fold or scan by a dfn runs along more than one vector, such as
each row of a matrix, it makes one batched call for each position along
the vectors. The call at position `j` runs on item `j` of every vector.
`{α-ω}/m` folds every row of `m` from the right with one call for each
column, rather than one for each item. In `x f.g y` with a dfn `g`, one
batched call runs `g` on every pairing of a row of `x`, a position along
the contracted axis and a column of `y`. `f` then folds along the
contracted axis. A fold or inner product stops its batched call for the
same reasons as a map, listed below, and then calls its operand once for
each item.

[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) in the
operand differentiates with respect to all the cells in one batched
call, cells that are arrays included: `{{+/ω*2}∂ω}¹ m` gives every row’s
gradient at once.

The map calls its operand once for each cell instead when the operand:

- reads a cell’s value to choose how many items to make or which items
  to read, as a count or a position does
- writes output, reads input, or formats a cell’s value with
  [`⍕`](https://answerdotai.github.io/basedpl/glyphs/thorn.html)
- calls a system function that acts
- writes to a name outside the map
- appends to a value that differs from cell to cell, or writes into one
  so that it would be held in [mixed
  storage](terms.qmd#term-mixed-storage)
- has an error guard that catches an error
- gives a result that holds arrays as items
- calls a primitive that can’t run on all the cells at once
- has a predicate that reads a cell’s value
- combines each cell with an array shared by every cell, when the shared
  array holds more than about a million items and repeating it for every
  cell would produce both more than about 2 million items and more than
  4 times the number of items in the call’s arguments
- has an argument that would exceed the size limit for one array if
  copied for every cell

When the shared array holds about a million items or fewer, the map
divides the cells into chunks and calls its operand once on each chunk.
Each call repeats the shared array into at most about 2 million items,
and the map joins the chunks’ results.

BPL records with each dfn why its batched call stopped, and later maps
of that dfn skip the batched call while that cause would recur. After
output, input,
[`⍕`](https://answerdotai.github.io/basedpl/glyphs/thorn.html), a system
function that acts, a write to a name outside the map or a read of a
cell’s value, later maps skip the batched call on every call. After a
call that can’t run on all the cells at once, later maps skip the
batched call while the arguments keep their kinds and cell shapes. After
a call that would repeat a shared array beyond the limit, later maps
skip the batched call while the arguments keep their kinds and cell
shapes and the frame keeps its shape. A caught error isn’t recorded,
because whether an error arises depends on the cells’ values.

A map always calls its operand once for each cell when an argument that
differs from cell to cell is held in mixed storage, as
[`•storage`](system/storage.qmd) reports for nested arrays and exact
rationals. A map of one cell whose arguments’ cells are atoms makes no
batched call either, because one plain call is faster. The `fallbacks`
table of [`•explain`](system/explain.qmd) lists, with the reason, each
map whose batched call fell back to one call for each cell, and each map
that skipped its batched call because an earlier one stopped. Its
`itemwise` table lists each batched call that read its cells’ items one
at a time. With `batch` set to `$f` in [`•prefs`](system/prefs.qmd),
every map calls its operand once for each cell.

## Errors

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Error</th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>DOMAIN</code></td>
<td>Invalid values or unknown inverse</td>
</tr>
<tr>
<td><code>RANK</code> / <code>LENGTH</code></td>
<td>Invalid rank or shape/count agreement</td>
</tr>
<tr>
<td><code>INDEX</code></td>
<td>A selector for an axis that doesn’t exist</td>
</tr>
<tr>
<td><code>SYNTAX</code></td>
<td>Invalid syntax or call form</td>
</tr>
<tr>
<td><code>VALUE</code></td>
<td>Undefined name or missing value</td>
</tr>
<tr>
<td><code>IO</code></td>
<td>A file, directory, URL or standard input that can’t be read, written
or reached</td>
</tr>
</tbody>
</table>

Each error report gives the error’s location in the source. Unsupported
features and resource limits have their own errors.
