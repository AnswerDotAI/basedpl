

# `⍤` — Atop / Rank

With function operand `g`, `f⍤g Y` is `f(g Y)` and `X f⍤g Y` is
`f(X g Y)`.

``` apl
2 -⍤+ 3            ⍝ ¯5
```

With numeric operand `R`, `f⍤R` applies to trailing cells of rank `R`.
Frames use leading agreement. Results assemble with fill. A space ends
the operand, so the argument needs no separator.

``` apl
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

Negative ranks count back from argument rank. Excessive ranks clamp.
Empty frames call the operand on prototype cells.

`∞` is the whole argument, and `¯∞` is rank 0. So `⍤1 ∞` pairs each row
on the left with the whole right argument. That gives the matrix-vector
product, which also multiplies matrices. A literal argument right after
a literal operand needs brackets, because a list of literals would take
it into the operand.

``` apl
m←[1 2 3 ⋄ 4 5 6]
m +⌿⍤×⍤1 ∞ [1 2 3]                   ⍝ 14 32
m +⌿⍤×⍤1 ∞ [1 10 ⋄ 2 20 ⋄ 3 30]      ⍝ [14 140 ⋄ 32 320]
```

To apply a function along chosen axes, see [Axis](axis.qmd).

## Errors

- `RANK`: a rank operand that is not a scalar or vector
- `LENGTH`: a rank operand with more than three items
- `DOMAIN`: non-integral ranks
