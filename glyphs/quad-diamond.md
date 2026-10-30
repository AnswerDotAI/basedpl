

# `⌺` — Stencil

Dyadic operator. Keys: `Alt-q Backtick`. Hanzi: `邻` (`鄰`)

`f⌺S Y` applies `f` to each neighbourhood of sizes `S` on the leading
axes of `Y`, centred on a position. Windows at the edges are padded with
prototype fill. It’s shorthand for Rank over
[Windows](up-down-arrow.qmd): `f⌺S Y` is `f⍤(-n) (-S)↕Y` for `n` sizes.
`f` gets the window alone, as J’s `u;._3` calls its operand.

``` bpl
+/⌺3 [1 2 3 4]                      ⍝ 3 6 9 7
(+/⌺3 [1 2 3 4])≡+/⍤¯1 [¯3]↕[1 2 3 4]   ⍝ $t
```

A two-row specification gives sizes, then movements. An options record,
as `↕` takes, can also name an edge mode, which a window’s positions
past an end read by. The edge mode is fill unless the record names
another. Its `fill` gives the item that padding with fill reads.

``` bpl
+/⌺[[3]⋄[2]]⍳8                       ⍝ 1 6 12 18
+/⌺["size":3 "edge":"wrap"] 1 2 3 4   ⍝ 7 6 9 8
+/⌺["size":3 "fill":100] 1 2 3 4      ⍝ 103 6 9 107
```

Results mix into one array, as Rank’s do. An argument that’s empty along
a windowed axis gives an empty result. The shape of each result comes
from one call of `f` on a window of fill, as Rank’s operand runs on
prototypes:

``` bpl
⍴{2⍴+/,⍵}⌺3⁝⍬   ⍝ [0 2]ₓ
```

A function right operand computes the specification from the argument:
`f⌺g Y` is `f⌺(g Y) Y`. [Rank](jot-diaeresis.qmd) and
[Axis](quad-colon.qmd) take a function right operand in the same way.
Here the window size is the square root of the length, rounded up:

``` bpl
+/⌺{⌈√≢⍵} ⍳9ₓ   ⍝ [1 3 6 9 12 15 18 21 15]ₓ
```

Where it can, Stencil calls `f` once on all the windows together, in a
[batched call](../evaluation.qmd#functions-and-effects), as Rank does.
The result is the same as from one call for each window.

## Errors

- `RANK`: specification not a unit, vector or two-row matrix, or more
  axes than `Y`
- `LENGTH`: a matrix without two rows
- `DOMAIN`: no axes, nonpositive sizes or movements, a size of at least
  twice the length of an axis that isn’t empty, an unknown option or
  edge mode, or a fill that doesn’t keep its value as `Y`’s type
