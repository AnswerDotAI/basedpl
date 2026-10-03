

# `⊂` — Enclose / Partitioned enclose

Keys: `Alt-z`

`⊂Y` is a scalar holding `Y`.

A one-item [bracket list](brackets.qmd), `[Y]`, is a vector, not a
scalar. `⊂` cannot enclose a function, because `⊂` next to a function
forms a train. To enclose a function, write
[`ᵘ`](../scripts.ipynb#superscripts) after it. `fᵘ` is a scalar holding
`f`.

``` bpl
⍴⊂1 2 3            ⍝ ⍬ₓ
↑⊂1 2 3            ⍝ 1 2 3
⊂3 ≡ 3             ⍝ $f
```

With [axes](axis.qmd), `⊂⍠K Y` encloses cells on axes `K`.

``` bpl
⊂⍠1 (2 3⍴⍳6)       ⍝ [[0 1 2] [3 4 5]]
```

`N⊂Y` starts partitions at positive marks along the leading axis. Each
partition is a block of major cells. Leading zero marks are ignored.

``` bpl
1 0 1 0⊂"abcd"     ⍝ "ab" "cd"
1 0 1⊂3 2⍴⍳6       ⍝ [[0 1 ⋄ 2 3] [4 5 ⋄]]
```

A mark greater than 1 starts extra empty partitions. Unit marks extend.
`N⊂⍠K Y` partitions along axis `K` instead. `N⊂⍤1 Y` partitions each
row.

``` bpl
1 0 1⊂⍠1 (2 3⍴⍳6)  ⍝ [[0 1 ⋄ 3 4] [[2] ⋄ [5]]]
1 0 1⊂⍤1 (2 3⍴⍳6)  ⍝ [[0 1] [2] ⋄ [3 4] [5]]
```

See also [`⊆`](nest.qmd).
