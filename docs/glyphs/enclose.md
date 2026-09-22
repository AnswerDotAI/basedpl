# `⊂` — Enclose / Partitioned enclose

`⊂Y` makes a rank-zero array containing `Y`.

```apl
⍴⊂1 2 3            ⍝ 0⍴0ₓ
↑⊂1 2 3            ⍝ 1 2 3
(⊂3)≡3             ⍝ 0ₓ
```

`⊂[K]Y` encloses cells on axes `K`.

```apl
⊂[2]2 3⍴⍳6         ⍝ (1 2 3⋄ 4 5 6)
```

`N⊂Y` starts partitions at positive marks along the last axis. Leading zero marks are ignored.

```apl
1 0 1 0⊂'abcd'     ⍝ 'ab' 'cd'
```

A mark greater than 1 starts extra empty partitions. Scalar marks extend; `[K]` selects the partition axis. See also [`⊆`](nest.md).
