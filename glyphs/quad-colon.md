

# `⍠` — Axis

Dyadic operator. Keys: `Alt-q ;`. Hanzi: `轴` (`軸`)

`f⍠A` applies `f` to the cells made of the axes `A`. The other axes form
the frame. `A` is a list of axes, given as numbers or names. Axes count
from 0, and negative axes count from the end. A keyed operand is an
options record, which [Fill](#fill) describes.

``` bpl
m←2 3⍴⍳6ₓ
+/⍠0 m                    ⍝ [3 5 7]ₓ
ax←0 ⋄ +/⍠ax m            ⍝ [3 5 7]ₓ
```

Numeric [subscripts](../scripts.ipynb#subscripts) give literal axes:
`+/₀` is `+/⍠0`, and `+/₁ ₂` is `+/⍠[1 2]`. Use `⍠` for names, computed
axes and options records.

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
⌽⍠0 m                     ⍝ [3 4 5⋄0 1 2]ₓ
⍴,⍠0 1⁝2 3 4⍴⍳24ₓ        ⍝ [6 4]ₓ
```

With two arguments, `A` applies to both. Cells pair using leading
agreement of the frames, as for [Rank](jot-diaeresis.qmd). An argument
without the selected axes is one whole cell, paired with every cell of
the other argument.

``` bpl
m←2 3⍴⍳6ₓ
1 0 1 #⍠1 m               ⍝ [0 2⋄3 5]ₓ
10 20 +⍠0 m               ⍝ [10 11 12⋄23 24 25]
```

A string names one axis, and a list of strings names several. A [named
index](iota.qmd), such as `⍳["row":2]`, names its own axis, and so does
an index from [`•index`](../system/index.qmd).

``` bpl
n←["row":2 "col":3]⍴⍳6ₓ
+/⍠"row" n                ⍝ ["col":3]⍴[3 5 7]ₓ
+/⍠"row" "col" n          ⍝ 15ₓ
r←⍳["row":2] ⋄ +/⍠r n     ⍝ ["col":3]⍴[3 5 7]ₓ
[b k]←•index 2 3 ⋄ +/⍠k b+k   ⍝ 3 6
```

A function right operand computes the axes from the arguments: `f⍠g Y`
is `f⍠(g Y) Y`, and `X f⍠g Y` is `X f⍠(X g Y) Y`.
[Rank](jot-diaeresis.qmd) and [Stencil](quad-diamond.qmd) take a
function right operand in the same way. Here the cells run along the
longest axis:

``` bpl
m←2 3⍴⍳6ₓ ⋄ +/⍠{↑⍒⍴⍵} m      ⍝ [3 12]ₓ
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
+/⍠1 2⁝2 3 4⍴⍳24ₓ        ⍝ [66 210]ₓ
```

Wrapping a primitive in a dfn gives the general rule:

``` bpl
{+/⍵}⍠1 2⁝2 3 4⍴⍳24ₓ     ⍝ [6 22 38⋄54 70 86]ₓ
```

## Inverse

`f⁻¹⍠a` inverts `f` along the same axes. The arithmetic and math
functions, comparisons, rotations, replicate and the catenates have such
inverses, and so do enclose, split, mix and reverse.

``` bpl
10 20+⁻¹⍠0[11 12 13⋄24 25 26]   ⍝ [1 2 3⋄4 5 6]
```

## Fill

A keyed operand is an options record, as in `f⍠["fill":v]`. Its one
option, `fill`, gives the item that `f` reads at each position that it
fills, in place of the [fill](../terms.qmd#term-fill) that the data’s
prototype gives. A pervasive call fills where one argument lacks a key
that the other has. Take fills past the end of an axis, replicate at a
negative count, expand at the items it inserts, and mix where it pads an
item. Index and pick fill at a NaN position. Pick and first also fill at
position 0 or ¯1 of an empty axis. Reshape fills where its argument is
empty. The fill reaches a primitive through a bound argument, as in
`3↑`, and through an axis operand, as in `↑⍠1`, including axes that a
function gives, as in `↑⍠{0}`. In `(f⍠R1)⍠R2`, `R1`’s fill applies. Any
other function fills no position and raises DOMAIN.

``` bpl
a←["p" "q"]:1 2 ⋄ b←["q" "r"]:3 4
a+b                             ⍝ ["p":1 "q":5 "r":4]
a+⍠["fill":10] b                ⍝ ["p":11 "q":5 "r":14]
5↑⍠["fill":¯1] 1 2 3            ⍝ 1 2 3 ¯1 ¯1
⊃⍠["fill":$n] [[1 2] [3]]         ⍝ [1 2⋄3 $n]
[[$n 1]]⌷⍠["fill":¯1] 10 20 30  ⍝ ¯1 20
(3↑⍠1)⍠["fill":¯1] [1 2⋄3 4]    ⍝ [1 2 ¯1⋄3 4 ¯1]
(3↑⍠{0})⍠["fill":¯1] [1 2⋄3 4]  ⍝ [1 2⋄3 4⋄¯1 ¯1]
((3↑)⍠["fill":¯1])⍠["fill":¯2] 1 ⍝ 1 ¯1 ¯1
```

The fill takes the type of the data that it fills. It never changes the
result’s type. A float fill rounds to the data’s float width. Integers,
Booleans and characters need a fill that they hold exactly. Under
dynamic kinds, ordinary integers widen to hold one. An array that holds
other arrays takes any fill.

``` bpl
5↑⍠["fill":255] [1 2 3]ₓ₁      ⍝ [1 2 3 255 255]ₓ₁
5↑⍠["fill":¯1000] [1 2 3]ₓ     ⍝ [1 2 3 ¯1000 ¯1000]ₓ
3↑⍠["fill":"NA"] "a" "bc"      ⍝ "a" "bc" "NA"
```

A call checks the fill when it starts, against each argument that it
could fill, even where it fills no position. For a pervasive call, those
are the arguments with a keyed axis. [Windows](up-down-arrow.qmd) and
[Stencil](quad-diamond.qmd) take a `fill` key in their own options
records. A fill given to them through `⍠` raises DOMAIN.

## Errors

- `RANK`: an axis operand that is not a unit or vector
- `DOMAIN`: axes that aren’t whole, repeated axes, axes outside the
  higher-rank argument, an options record with a key other than `fill`,
  a function that fills no position, Windows or Stencil, or a fill that
  doesn’t keep its value as the data’s type
