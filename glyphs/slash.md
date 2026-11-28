

# [`/`](https://answerdotai.github.io/basedpl/glyphs/slash.html) — Reduce

Monadic operator. Hanzi: `总` (`總`)

`f/Y` inserts `f` between the items along the last axis, then evaluates
that expression as usual, from the right. `-/1 2 3` is `1-2-3`, which is
2. An empty axis reduces to `f`’s identity.

``` bpl
+/1 2 3            ⍝ 6
-/1 2 3            ⍝ 2
+/⍬                ⍝ 0
```

Reduce returns the last result of `f`, not enclosed. An unseeded
one-item reduction returns that item, at the type that a longer
reduction gives: `+/,$t` counts one item as `+/$t $f` counts two.
Unreduced axes form the result frame.

A dfn that reduces several cells, such as the rows of a matrix, runs on
every cell at once, in one [batched
call](../evaluation.qmd#functions-and-effects) for each position along
the axis.

``` bpl
,/⊂2 3            ⍝ 2 3
,/,⊂2 3           ⍝ 2 3
+/⊂⊂3             ⍝ ⊂3
+/,$t              ⍝ 1ₓ
```

`S f/ Y` starts from seed `S` on the right. With `Y` holding `[a b c]`,
it is `a f b f c f S`. Empty reductions return the seed. Each cell uses
the whole seed. The seed can share a run with the reduction, as in
`10-/1 2 3`. With a space, `10 -/1 2 3` is the strand `10 2`.

``` bpl
10-/1 2 3          ⍝ ¯8
10+/2 3ρι6ₓ         ⍝ [13 22]
⊂10 +/ 1 2 3       ⍝ ⊂16
10 20 +/⁰ ¹ [2 3]ρι6ₓ ⍝ [13 32]
```

Reduce keeps fixed-width integers at their fixed type, promoted with any
seed. A wider seed widens the running result without widening the stored
input array:

``` bpl
v←[200 100]ₓ₁
+/v                 ⍝ 44ₓ₁
+/vₓ                ⍝ 300ₓ
0ₓ₈+/v              ⍝ 300ₓ₈
(+/)ₓ₈ v            ⍝ 44ₓ₈
```

For an ordinary exact total, use
[`+/vₓ`](../numbers.qmd#fixed-width-integers). The fixed-i64 seed makes
each addition i64. The seed is promoted with the input, and cannot
narrow wider input. Write `0ₓ₈+/v` or `0ₓ₈ +/ v`, not `0ₓ₈ +/v`, which
is the strand of `0ₓ₈` and `+/v`. A result suffix converts the completed
total and cannot recover wrapped values.

A seeded Pick reduction follows a nested path.

``` bpl
tree←[[10 20] [30 [40 50]]]
tree⊃/⌽1 1 0       ⍝ 40
```

Use [full windows](up-down-arrow.qmd) for moving reductions.

``` bpl
+/↕1 2 3 4         ⍝ 3 5 7
-/⌽↕1 2 3          ⍝ 1 1
+/0↕1 2            ⍝ 0 0 0
```

`f/⍠K` selects the axis. [`⌿`](slash-bar.qmd) defaults to the first.

Several axes form one reduction cell. `f/⍠A Y` ravels each selected cell
before reducing, equivalent to `(f/,)¨⊂⍠A Y`. This is an exception to
the general rule in [Axis](quad-colon.qmd). With a single axis, Reduce
works as above.

``` bpl
n←2 2 3ρι6ₓ
+/₁ ₂n               ⍝ [15 15]ₓ
+/₀ ₂n               ⍝ [6 24]ₓ
```

Axis order determines the ravel order. Unselected axes retain their
order and keys. Selecting every axis gives the reduced value itself, not
enclosed, as a dfn along the same axes does.

``` bpl
-/₀ ₁ 2 2ρι4ₓ         ⍝ ¯2ₓ
-/₁ ₀ 2 2ρι4ₓ         ⍝ ¯4ₓ
```

With an empty axis vector, each item is its own cell. A seed is passed
whole to each cell.

``` bpl
+/⍠⍬ [1 2 3]          ⍝ 1 2 3
10+/₁ ₂ 2 2 3ρι6ₓ      ⍝ [25 25]
```

Algebraic reductions such as `+/` can reassociate their operands, as
[Accuracy](../numbers.qmd#accuracy) describes. Other reductions keep
operand order.

## Errors

- `RANK`: higher-rank axis specification
- `DOMAIN`: axis vector with repeated or invalid axes, or a subject
  operand. Replicate with [`#`](hash.qmd)
