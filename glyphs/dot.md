

# `.` — Determinant / Inner product

Dyadic operator. Hanzi: `内` (`內`)

`X f.g Y` pairs items with `g`, then reduces with `f`. It contracts the
last axis of `X` with the first of `Y`.

``` bpl
1 2 3+.×4 5 6      ⍝ 32
[1 0⋄0 1]+.×2 2⍴⍳4ₓ ⍝ 2 2⍴⍳4
```

Singleton contraction axes extend. Empty contractions use the reduction
identity. `g⊗` is [outer product](circle-times.qmd).

Float and complex `+.×` read a large operand that is a transpose, or
windows of an array, without copying it. A transpose goes to the matrix
product where it lies. Windows are read a block of rows at a time, as
[Windows](up-down-arrow.qmd) describes.

Where `g` is a dfn, `X f.g Y` calls it once on all the pairings
together, in a [batched call](../evaluation.qmd#functions-and-effects).
The result is the same as from one call for each pairing.

`f.g Y`, with no left argument, expands the matrix `Y` by minors along
its first column, as J’s `u .v` does. `-.×` gives the determinant, and
`+.×` the permanent.

``` bpl
-.×[1 2⋄3 4]ₓ      ⍝ ¯2ₓ
+.×[1 2⋄3 4]ₓ      ⍝ 10ₓ
⌈.⌊[1 5⋄4 2]       ⍝ 4
```

The expansion is `C f.g D`, where `C` is the matrix’s first column and
`D` holds the expansions of its minors. Each minor leaves out the first
column and one row. A minor that the expansion meets more than once is
expanded only once. `f` and `g` run once for each distinct minor. A
matrix with no columns gives `g/` of no items, `g`‘s identity. Every
expansion ends there. `g` therefore needs an identity, as in J. A vector
counts as one column. A number counts as a 1-by-1 matrix. Over every way
to pick one item from each row and column, `⌈.⌊` gives the largest of
the picks’ smallest items. The expansion takes time that doubles with
each row. It accepts at most 64 rows.

After an array, `.name` reads a [keyed array](../keyed.ipynb) value:
`T.name` is `"name"⊃T`. It also works as an assignment target. Plain
assignment creates missing records along the path.

``` bpl
T←["n":1 "addr":["city":"LA"]]
T.addr.city        ⍝ "LA"
T.n+←1 ⋄ T.n       ⍝ 2
```

After an array, `.` followed by brackets indexes it. Each bracket item
is a position, an array of positions, `:` for one whole input axis, `∞`
for all remaining axes here, or `¯∞` for a new length-one axis. The new
axis consumes no input axis and has no name or keys. The positions on
one axis form one item, as in `m.[[2 0]]`. Arrays of positions pair with
each other by broadcasting, as arithmetic pairs its arguments, and as
NumPy’s advanced indexing pairs index arrays. Named arrays follow
arithmetic’s rule for names: arrays with the same name pair, and arrays
with different names build a grid. A named array whose name no other
array has selects along the axis with its name. Arrays that share a name
take the free axes in order, as [Index](squad.qmd) describes. A NaN
position reads the array’s fill, and [`•edge`](../system/edge.qmd) gives
NaN for a position outside its axis. The result holds one item for each
pair, on axes that take the place of the first array. Each `:` keeps its
axis in place, a single position removes its axis, and axes after the
last item stay whole. With at most one array, `x.[I]` reads what `[I]⌷x`
reads. An array whose items are coordinates, as `⍸` gives them, covers
the axes its coordinates span and pairs with the other arrays as one
array, as [Index](squad.qmd) describes. [Index](squad.qmd) gives the
cross product of several arrays, and a path through nested items is
[Pick](right-shoe.qmd). Dot indexing binds as tightly as `.name` and
needs no parentheses inside a larger expression. Paths chain from left
to right. The array is evaluated before the index.
[Indexing](../indexing.ipynb) shows everyday uses.

``` bpl
m←3 4⍴⍳12ₓ
m.[1 2]            ⍝ 6ₓ
m.[[2 0]]          ⍝ [8 9 10 11⋄0 1 2 3]ₓ
m.[⍳2ₓ ¯1]          ⍝ [3 7]ₓ
m.[[0 2] [1 3]]    ⍝ [1 11]ₓ
m.[: [0 2]]        ⍝ [0 2⋄4 6⋄8 10]ₓ
i←⍳3ₓ ⋄ (3 3⍴⍳9ₓ).[i i]  ⍝ [0 4 8]ₓ
[r c]←•index 3 4 ⋄ m≡m.[r c]   ⍝ $t
m.[[1 $n]]         ⍝ [4 5 6 7⋄0 0 0 0]ₓ
m.[⍸m>9]           ⍝ [10 11]ₓ
9,m.[0 1],3        ⍝ 9 1 3
```

After a function, `.name` and `.[I]` read from the function’s result, as
they read from an array: `f.name` is `{(⍺ f ⍵).name}`, and `f.[I]` is
`{(⍺ f ⍵).[I]}`, with one argument or two. A path goes on from there, as
in `⊢.x.[0]`. A name that holds a function, or a path to one, gives an
inner product instead, as in `+.ops.mul`. [Under](at.qmd) writes back
through the read.

``` bpl
v←10 20 30 40
⊢.[[1 3]] v             ⍝ 20 40
1 2 3 4 +.[[0 2]] v     ⍝ 11 33
{⍵×10}@ ⊢.[[1 3]] v     ⍝ 10 200 30 400
T←["a":1 "x":[10 20 30]]
⊢.a¨[T T]               ⍝ 1 1
⊢.x.[[2 0]] T           ⍝ 30 10
```

After a keyed vector `T`, `T.(expr)` evaluates `expr` with each key of
`T` as a name for its item. After a matrix, or any array with keys on
its first axis, each key names its major cell. Names that aren’t keys
resolve in the current scope, and a key hides a name it shares. A key
that holds a function binds as a function. Assignments inside the
parentheses stay there, and leave `T` and the current scope unchanged.
[Execute](hydrant.qmd) does the same for code in text: `T⍎"price×qty"`
is `T.(price×qty)`. A scope doesn’t map over an array of records. Write
`{⍵.(expr)}¨recs` for that.

``` bpl
T←["price":2 3;"qty":10 20]
T.(price×qty)              ⍝ 20 60
rate←0.5 ⋄ T.(price×rate)  ⍝ 1 1.5
r←T.(price←0) ⋄ T.price    ⍝ 2 3
M←"price" "qty":[2 3⋄10 20]
M.(price×qty)              ⍝ 20 60
```

A dot followed by a digit is a decimal point: `.2` is a number. To index
position 2, write `v.[2]` or the
[subscript](../scripts.ipynb#subscripts) `v₂`. Assignment through a dot
path writes the items it reads, and adds missing keys.

``` bpl
v←10 20 30
v.[[2 0]]←7 8 ⋄ v  ⍝ 8 20 7
```

## Fast combinations

These combinations give the result of the expansion, faster.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Combination</th>
<th>Applies to</th>
<th>Benefit</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>-.×Y</code></td>
<td>a square matrix of numbers</td>
<td>takes one LU factorization for floats and complex numbers, or row
reduction for exact numbers, which keeps the result exact</td>
</tr>
<tr>
<td><code>+.×Y</code></td>
<td>a square matrix of numbers with fewer than 64 rows</td>
<td>takes rustymath’s permanent, without the expansion’s arrays of
minors</td>
</tr>
</tbody>
</table>

``` bpl
m←[2 1 0⋄1 3 1⋄0 1 4]
(•explain "-.× m").fast.combination  ⍝ ["determinant"]
(•explain "+.× m").fast.combination  ⍝ ["permanent"]
```
