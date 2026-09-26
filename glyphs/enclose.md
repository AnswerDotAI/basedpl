

# `⊂` — Enclose / Partitioned enclose

`⊂Y` makes a rank-zero array containing `Y`.

`⊂` is the only way to enclose an array: [brackets](brackets.qmd) round
one item only group it. `⊂` cannot enclose a function, because `⊂` next
to a function forms a train. A one-item list holds a function as a value
instead: `[+/÷≢;]`.

``` apl
⍴⊂1 2 3            ⍝ 0⍴0ₓ
↑⊂1 2 3            ⍝ 1 2 3
(⊂3)≡3             ⍝ 0ₓ
```

With [axes](axis.qmd), `⊂⍠K Y` encloses cells on axes `K`.

``` apl
⊂⍠1 (2 3⍴⍳6)       ⍝ [[0 1 2] [3 4 5]]
```

`N⊂Y` starts partitions at positive marks along the last axis. Leading
zero marks are ignored.

``` apl
1 0 1 0⊂"abcd"     ⍝ "ab" "cd"
```

A mark greater than 1 starts extra empty partitions. Scalar marks
extend. `N⊂⍠K Y` partitions along axis `K`, cutting the whole array into
blocks. See also [`⊆`](nest.qmd).
