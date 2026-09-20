# `/` — Replicate / Reduce

`N/Y` repeats last-axis items by their counts. Boolean counts filter.

```apl
1 0 1/10 20 30     ⍝ 10 30
2 1/10 20          ⍝ 10 10 20
```

Negative counts insert fill. Singleton counts or a singleton axis extend.

```apl
1 ¯2/10 20         ⍝ 10 0 0
2/3 4              ⍝ 3 3 4 4
1 0 1/,3           ⍝ 3 3
```

`f/Y` reduces the last axis, right-associated. Empty axes use `f`'s identity.

```apl
+/1 2 3            ⍝ 6
-/1 2 3            ⍝ 2
+/⍬                ⍝ 0
```

Reduction returns the accumulator. An unseeded one-item reduction returns that item. Unreduced axes supply the result frame.

```apl
,/⊂2 3            ⍝ 2 3
,/,⊂2 3           ⍝ 2 3
+/⊂⊂3             ⍝ ⊂3
```

`S f/Y` starts from seed `S` on the right: `S f/a b c` is `a f b f c f S`. Empty reductions return the seed. Each lane uses the whole seed.

```apl
10 -/1 2 3         ⍝ ¯8
10 +/2 3⍴⍳6        ⍝ 16 25
(⊂10)+/1 2 3       ⍝ ⊂16
10 20 (+/⍤0 1)2 3⍴⍳6 ⍝ 16 35
```

Nested paths use seeded Pick reduction.

```apl
tree←(10 20⋄ 30 (40 50))
tree⊃/⌽2 2 1       ⍝ 40
```

Use [full windows](windows.md) for moving reductions.

```apl
+/2↕1 2 3 4        ⍝ 3 5 7
-/⌽2↕1 2 3         ⍝ 1 1
+/0↕1 2            ⍝ 0 0 0
```

`[K]` selects the axis. [`⌿`](slash-bar.md) defaults to the first. Float sum/product reductions allow reassociation; generic reductions retain operand order.
