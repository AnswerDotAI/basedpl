

# [`⍤`](https://answerdotai.github.io/basedpl/glyphs/jot-diaeresis.html) — Rank

Dyadic operator. Keys: `Alt-1 t`. Hanzi: `秩`

Numeric [superscripts](../scripts.ipynb#on-a-function) are the short
form: `f¹` is `f⍤1`, `f⁰ ¹` is `f⍤[0 1]`, and `f⁰ ¹ ²` is `f⍤[0 1 2]`.
Adjacent digits make one rank (`f¹²` means rank 12); spaces separate
ranks. `f⁻¹` is rank ¯1, whereas bare `f⁻` is
[inverse](superscript-minus.qmd) and `f⁻ ¹` applies that inverse at
rank 1. Use
[`⍤`](https://answerdotai.github.io/basedpl/glyphs/jot-diaeresis.html)
for named, computed or infinite ranks. A numeric argument needs no
separator after superscripts: `-⁰5`.

`f⍤R` applies `f` to the [cells](../terms.qmd#term-cell) of rank `R`.
With two arguments, `f⍤R` pairs cells using [leading
agreement](../evaluation.qmd#agreement-and-pervasion) of the
[frames](../terms.qmd#term-frame). The [token separator
`⁝`](tricolon.qmd) keeps a numeric argument apart from the rank, as in
`-⍤0⁝5`. Put a vector argument in brackets that touch the rank, as in
`-⍤0[2 3]`. In `-⍤0 5`, `0 5` is the rank. Three rules give the result:

1.  Inside a frame, the 0-cell at a position holding an atom is that
    atom. The 0-cell at a position holding an array is a scalar holding
    the array.
2.  [Assembly](../terms.qmd#term-assembly) works as for
    [`⊃`](https://answerdotai.github.io/basedpl/glyphs/right-shoe.html).
    Shorter results are padded with fill to a common shape. The shape of
    the whole result is the frame followed by that common shape. Each
    position in the frame holds the items of its result.
3.  When no argument has a frame, `f⍤R` is the same as `f`. An argument
    has no frame when its rank is at most its cell rank. An atom never
    has a frame.

``` bpl
m←2 3ρι6ₓ
+/¹ m             ⍝ [3 12]ₓ
```

<table>
<thead>
<tr>
<th><code>R</code></th>
<th>Monad rank</th>
<th>Dyad left/right ranks</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>r</code></td>
<td><code>r</code></td>
<td><code>r r</code></td>
</tr>
<tr>
<td><code>q r</code></td>
<td><code>r</code></td>
<td><code>q r</code></td>
</tr>
<tr>
<td><code>p q r</code></td>
<td><code>p</code></td>
<td><code>q r</code></td>
</tr>
</tbody>
</table>

A negative rank counts back from the argument’s rank. With a rank at
least as large as the argument’s rank, the whole argument is one cell.
With an empty frame, `f` is called on a cell filled with the argument’s
[prototype](../terms.qmd#term-prototype). If that call fails, the result
is empty, with the frame’s shape and prototype 0, as in J.

`⊢⍤0` returns its argument unchanged. On a [simple
array](../terms.qmd#term-simple-array), `f⍤0` is `⊃f¨`.
[Each](diaeresis.qmd) passes the items themselves, never enclosed. Every
call’s result becomes one item of Each’s result, with no assembly.

``` bpl
v←[[1 2] [3 4 5]]
≢⍤0 v              ⍝ [1 1]ₓ
≢¨v                ⍝ [2 3]ₓ
⊢⍤0v ≡ v          ⍝ $t
,⍨⍤0[1 2]         ⍝ [1 1◇2 2]
-⍤0⁝5            ⍝ ¯5
```

With rank
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html), the
cell is the whole argument, and rank `¯∞` is rank 0. For example, `⍤1 ∞`
pairs each row on the left with the whole right argument. In the
examples below, `+⌿× ⍤1 ∞` is the matrix-vector product. The space gives
Rank the whole train `+⌿×` as its left operand. With a matrix on the
right, the same function multiplies matrices.

``` bpl
m←[1 2 3◇4 5 6]
m +⌿× ⍤1 ∞ [1 2 3]                   ⍝ 14 32
m +⌿× ⍤1 ∞ [1 10◇2 20◇3 30]      ⍝ [14 140◇32 320]
```

A function right operand computes the ranks from the arguments, as BQN’s
`⎉` does: `f⍤g Y` is `f⍤(g Y) Y`, and `X f⍤g Y` is `X f⍤(X g Y) Y`.
[Axis](quad-colon.qmd) and [Stencil](quad-diamond.qmd) take a function
right operand in the same way. Here each cell of `Y` with the rank of
`X` pairs with `X`:

``` bpl
[1 2] +⍤{≢ρα} [10 20◇30 40]   ⍝ [11 22◇31 42]
```

To apply a function along chosen axes, see [Axis](quad-colon.qmd).

## Calls on the whole argument

Rank has whole-argument routes for these functions:

- a pervasive function, as in `m +⍤1 v`, where no frame axis has a name
- a fold or scan by a pervasive primitive on a simple array, as in
  `+/⍤1 m`, which runs as `+/m` does
- a monadic primitive at or above the rank that its page gives, such as
  `⌽⍤1 m`, except
  [`ι`](https://answerdotai.github.io/basedpl/glyphs/iota.html),
  [`↦`](https://answerdotai.github.io/basedpl/glyphs/right-arrow-from-bar.html)
  and [`⍎`](https://answerdotai.github.io/basedpl/glyphs/hydrant.html)
- `⌽₀`, [`,`](https://answerdotai.github.io/basedpl/glyphs/comma.html),
  [`↑`](https://answerdotai.github.io/basedpl/glyphs/up-arrow.html),
  [`↓`](https://answerdotai.github.io/basedpl/glyphs/down-arrow.html),
  [`⌷`](https://answerdotai.github.io/basedpl/glyphs/squad.html),
  [`⍉`](https://answerdotai.github.io/basedpl/glyphs/circle-backslash.html)
  and [`#`](https://answerdotai.github.io/basedpl/glyphs/hash.html)
  along the cells’ axes,
  [`≢`](https://answerdotai.github.io/basedpl/glyphs/equal-underbar-slash.html)
  and [`ρ`](https://answerdotai.github.io/basedpl/glyphs/rho.html), and
  a reshape that keeps each cell’s items in order
- [`,`](https://answerdotai.github.io/basedpl/glyphs/comma.html) and
  [`⍪`](https://answerdotai.github.io/basedpl/glyphs/comma-bar.html),
  joining each pair of cells, where the two arguments’ frames match
- a dyadic primitive at or above the ranks that its page gives, where
  the two arguments’ frames match, such as `v ⌽⍤0 1 m`, except
  [`↦`](https://answerdotai.github.io/basedpl/glyphs/right-arrow-from-bar.html),
  [`:`](https://answerdotai.github.io/basedpl/glyphs/colon.html),
  [`⊂`](https://answerdotai.github.io/basedpl/glyphs/left-shoe.html),
  [`⊃`](https://answerdotai.github.io/basedpl/glyphs/right-shoe.html),
  [`⊆`](https://answerdotai.github.io/basedpl/glyphs/left-shoe-underbar.html)
  and [`⍕`](https://answerdotai.github.io/basedpl/glyphs/thorn.html)
- `+.×`,
  [`⌹`](https://answerdotai.github.io/basedpl/glyphs/domino.html), `-.×`
  and [`•decompose`](../system/decompose.qmd) on a stack of matrices
- an atop `f∘g` where `g` runs as one call, with Rank applying `f` to
  the cells of its result

Rank calls any other function, a dfn included, once on all the cells
together where it can, in a [batched
call](../evaluation.qmd#functions-and-effects). Otherwise it calls the
function once for each cell.

## Inverse

`(f⍤k)⁻` is `f⁻⍤k`.

``` bpl
(⌽⍤1)⁻[1 2◇3 4]   ⍝ [2 1◇4 3]
```

## Errors

- `RANK`: a rank operand that is not a unit or vector
- `LENGTH`: a rank operand with more than three items
- `DOMAIN`: ranks that aren’t whole
