

# `⍠` — Axis

Dyadic operator. Keys: `Alt-q ;`. Hanzi: `轴` (`軸`)

`f⍠A` applies `f` to the cells made of the axes `A`. The other axes form
the frame. `A` is a list of axes, given as numbers or names. Axes count
from 0, and negative axes count from the end.

``` bpl
m←2 3⍴⍳6ₓ
{+/⍵}⍠0 m                 ⍝ [3 5 7]ₓ
ax←0 ⋄ {+/⍵}⍠ax m         ⍝ [3 5 7]ₓ
```

Rank is the case that selects the last `r` axes. So `⍤1` and `⍠¯1` both
select the last axis.

``` bpl
m←2 3⍴⍳6ₓ
+/⍤1 m                    ⍝ [3 12]ₓ
+/⍠¯1 m                   ⍝ [3 12]ₓ
+/⍠0 m                    ⍝ [3 5 7]ₓ
```

Result cells with the same rank as the selected cells return to the axes
`A`. Other result cells start at the position of the first selected
axis.

``` bpl
m←2 3⍴⍳6ₓ
{⌽⍵}⍠0 m                  ⍝ [3 4 5⋄0 1 2]ₓ
⍴{,⍵}⍠0 1 (2 3 4⍴⍳24ₓ)     ⍝ [6 4]ₓ
```

With two arguments, `A` applies to both. Cells pair using leading
agreement of the frames, as for [Rank](jot-diaeresis.qmd). An argument
without the selected axes is one whole cell, paired with every cell of
the other argument.

``` bpl
m←2 3⍴⍳6ₓ
1 0 1 {⍺#⍵}⍠1 m           ⍝ [0 2⋄3 5]ₓ
10 20 {⍺+⍵}⍠0 m           ⍝ [10 11 12⋄23 24 25]
```

A string names one axis, and a list of strings names several. A [named
index](iota.qmd), such as `⍳["row":2]`, names its own axis, and so does
an index from [`•index`](../system/index.qmd).

``` bpl
n←["row":2 "col":3]⍴⍳6ₓ
{+/⍵}⍠"row" n             ⍝ ["col":3]⍴[3 5 7]ₓ
{+/,⍵}⍠"row" "col" n      ⍝ 15ₓ
r←⍳["row":2] ⋄ {+/⍵}⍠r n  ⍝ ["col":3]⍴[3 5 7]ₓ
[b k]←•index 2 3 ⋄ +/⍠k b+k   ⍝ 3 6
```

A function right operand computes the axes from the arguments: `f⍠g Y`
is `f⍠(g Y) Y`, and `X f⍠g Y` is `X f⍠(X g Y) Y`.
[Rank](jot-diaeresis.qmd) and [Stencil](quad-diamond.qmd) take a
function right operand in the same way. Here the cells run along the
longest axis:

``` bpl
m←2 3⍴⍳6ₓ ⋄ {+/⍵}⍠{↑⍒⍴⍵} m   ⍝ [3 12]ₓ
```

## Exceptions

A primitive with its own axis meaning keeps it:

- Multi-axis reduce, including seeded reduce. `+/⍠1 2` flattens each
  cell before reducing.
- Partitioned enclose and partition. `1 0 1 0⊂⍠1` and `1 1 2 2⊆⍠1` cut
  the whole matrix into blocks along axis 1. `⊂⍠K` matches Dyalog.
  Dyalog’s `⊆[K]` partitions each lane instead, which is `{⍺⊆⍵}⍠K` here.
- Rotate with a count array. `1 2 3 4⌽⍠0` pairs one count with each
  column.
- Mix. `⊃⍠A` places the cell axes among the frame axes.
- Axis selectors. `⍳⍠A` returns the keys or positions of the selected
  axes.

``` bpl
+/⍠1 2 (2 3 4⍴⍳24ₓ)        ⍝ [66 210]ₓ
```

Wrapping a primitive in a dfn gives the general rule:

``` bpl
{+/⍵}⍠1 2 (2 3 4⍴⍳24ₓ)     ⍝ [6 22 38⋄54 70 86]ₓ
```

## Inverse

`f⁻¹⍠a` inverts `f` along the same axes. The arithmetic and math
functions, comparisons, rotations, replicate and the catenates have such
inverses, and so do enclose, split, mix and reverse.

``` bpl
10 20+⁻¹⍠0[11 12 13⋄24 25 26]   ⍝ [1 2 3⋄4 5 6]
```

## Errors

- `RANK`: an axis operand that is not a unit or vector
- `DOMAIN`: non-integral axes, repeated axes, axes outside the
  higher-rank argument
