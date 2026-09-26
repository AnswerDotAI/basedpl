

# `/` — Replicate / Reduce

`N/Y` repeats last-axis items by their counts. Boolean counts filter.

``` apl
1 0 1/10 20 30     ⍝ 10 30
2 1/10 20          ⍝ 10 10 20
```

Negative counts insert fill. Singleton counts or a singleton axis
extend.

``` apl
1 ¯2/10 20         ⍝ 10 0 0
2/3 4              ⍝ 3 3 4 4
1 0 1/,3           ⍝ 3 3
```

`f/Y` reduces the last axis, right-associated. Empty axes use `f`’s
identity.

``` apl
+/1 2 3            ⍝ 6
-/1 2 3            ⍝ 2
+/⍬                ⍝ 0
```

Reduction returns the accumulator. An unseeded one-item reduction
returns that item. Unreduced axes supply the result frame.

``` apl
,/⊂2 3            ⍝ 2 3
,/,⊂2 3           ⍝ 2 3
+/⊂⊂3             ⍝ ⊂3
```

`S f/ Y` starts from seed `S` on the right. With `Y` holding `[a b c]`,
it is `a f b f c f S`. Empty reductions return the seed. Each lane uses
the whole seed. The seed can share a unit with the reduction, as in
`10-/1 2 3`. In `10 -/1 2 3`, the unit `-/1 2 3` is a value, so `10` is
applied to it as an array.

``` apl
10-/1 2 3          ⍝ ¯8
10+/2 3⍴⍳6         ⍝ 13 22
(⊂10)+/1 2 3       ⍝ ⊂16
10 20(+/⍤0 1)2 3⍴⍳6 ⍝ 13 32
```

Nested paths use seeded Pick reduction.

``` apl
tree←[[10 20] [30 [40 50]]]
tree⊃/⌽1 1 0       ⍝ 40
```

Use [full windows](windows.qmd) for moving reductions.

``` apl
+/2↕1 2 3 4        ⍝ 3 5 7
-/⌽2↕1 2 3         ⍝ 1 1
+/0↕1 2            ⍝ 0 0 0
```

`f/⍠K` and `N/⍠K` select the axis. [`⌿`](slash-bar.qmd) defaults to the
first.

Several axes form one reduction cell. `f/⍠A Y` ravels each selected cell
before reducing, equivalent to `(f/⍤,)¨⊂⍠A Y`. This is an exception to
the general rule in [Axis](axis.qmd). A single axis retains ordinary
reduction behavior.

``` apl
n←2 2 3⍴⍳6
+/⍠1 2 n             ⍝ 15 15
+/⍠0 2 n             ⍝ 6 24
```

Axis order determines the ravel order. Unselected axes retain their
order and keys. Selecting every axis gives the reduced value itself, as
a dfn along the same axes does.

``` apl
-/⍠0 1 (2 2⍴⍳4)      ⍝ ¯2
-/⍠1 0 (2 2⍴⍳4)      ⍝ ¯4
```

An empty axis vector makes singleton cells. A seed is passed whole to
each cell.

``` apl
+/⍠⍬ (1 2 3)          ⍝ 1 2 3
10+/⍠1 2 (2 2 3⍴⍳6)   ⍝ 25 25
```

Float sum/product reductions allow reassociation; generic reductions
retain operand order.

## Errors

- `RANK`: higher-rank axis specification
- `DOMAIN`: axis vector with repeated or invalid axes
