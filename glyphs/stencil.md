

# `⌺` — Stencil

Keys: `Alt-q Backtick`

`f⌺S Y` applies `f` to neighbourhoods of sizes `S` on leading axes.
Windows at the edges are padded with prototype fill.

Negative sizes in [Windows](windows.qmd) return the same windows as one
array.

``` bpl
(+/⊢)⌺3 [1 2 3 4]     ⍝ 3 6 9 7
```

`⍵` is the window. `⍺` gives signed padding counts per axis: positive at
the start, negative at the end. The two-chain `(+/⊢)` discards `⍺`. A
two-row specification gives sizes then movements.

``` bpl
(+/⊢)⌺[[3]⋄[2]]⍳8  ⍝ 1 6 12 18
```

Results assemble with fill.

A function right operand computes the specification from the argument:
`f⌺g Y` is `f⌺(g Y) Y`. [Rank](rank.qmd) and [Axis](axis.qmd) take a
function right operand in the same way. Here the window size is the
square root of the length, rounded up:

``` bpl
(+/⊢)⌺{⌈√≢⍵} ⍳9   ⍝ 1 3 6 9 12 15 18 21 15
```

## Errors

- `RANK`: specification not a unit, vector or two-row matrix, or more
  axes than `Y`
- `LENGTH`: a matrix without two rows
- `DOMAIN`: no axes, nonpositive sizes or movements, or a size of at
  least twice its axis length
