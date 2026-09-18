# `⊆` — Nest / Partition

`⊆Y` encloses a simple non-scalar array; scalars and already-nested arrays stay unchanged.

```apl
⊆1 2               ⍝ ⊂1 2
⊆1                 ⍝ 1
```

`N⊆Y` partitions along the last axis: zero omits, a rise in the mark starts a partition.

```apl
1 1 0 1 1⊆'abcde' ⍝ 'ab' 'de'
```

Marks are nonnegative integers. Scalar/singleton marks extend. `[K]` selects the axis. Unlike [`⊂`](enclose.md), positive marks need not start a partition at every position.
