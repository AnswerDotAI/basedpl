

# `⍤` — Rank

Keys: `Alt-1 t`

`f⍤R` applies `f` to the [cells](../terms.qmd#term-cell) of rank `R`.
With two arguments, `f⍤R` pairs cells using leading agreement of the
[frames](../terms.qmd#term-frame). Write a literal argument after the
rank in brackets or [parentheses](parentheses.qmd), as in `-⍤0(5)`.
Without brackets or parentheses, `0 5` in `-⍤0 5` is the rank. Three
rules give the result:

1.  Inside a frame, the 0-cell at a position holding an atom is that
    atom. The 0-cell at a position holding an array is a scalar holding
    the array.
2.  [Assembly](../terms.qmd#term-assembly) works as for `⊃`. Shorter
    results are padded with fill to a common shape. The shape of the
    whole result is the frame followed by that common shape. Each
    position in the frame holds the items of its result.
3.  When no argument has a frame, `f⍤R` is the same as `f`. An argument
    has no frame when its rank is at most its cell rank. An atom never
    has a frame.

``` bpl
m←2 3⍴⍳6
+/⍤1 m             ⍝ 3 12
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
With an empty frame, `f` is called on a prototype cell.

By these rules, `⊢⍤0` returns its argument unchanged. On an array of
atoms, `f⍤0` is `⊃f¨`. [Each](each.qmd) passes the items themselves,
never enclosed. Every call’s result becomes one item of Each’s result,
with no assembly.

``` bpl
v←[[1 2] [3 4 5]]
≢⍤0 v              ⍝ [1 1]ₓ
≢¨v                ⍝ [2 3]ₓ
(⊢⍤0 v)≡v          ⍝ $t
{⍵ ⍵}⍤0[1 2]      ⍝ [1 1 ⋄ 2 2]
-⍤0(5)             ⍝ ¯5
```

With rank `∞`, the cell is the whole argument, and rank `¯∞` is rank 0.
For example, `⍤1 ∞` pairs each row on the left with the whole right
argument. In the examples below, `+⌿∘×⍤1 ∞` is the matrix-vector
product. With a matrix on the right, the same function multiplies
matrices.

``` bpl
m←[1 2 3 ⋄ 4 5 6]
m +⌿∘×⍤1 ∞ [1 2 3]                   ⍝ 14 32
m +⌿∘×⍤1 ∞ [1 10 ⋄ 2 20 ⋄ 3 30]      ⍝ [14 140 ⋄ 32 320]
```

To apply a function along chosen axes, see [Axis](axis.qmd).

## Errors

- `RANK`: a rank operand that is not a unit or vector
- `LENGTH`: a rank operand with more than three items
- `DOMAIN`: non-integral ranks
