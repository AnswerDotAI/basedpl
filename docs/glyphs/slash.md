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

`N f/Y` reduces overlapping windows of width `|N|`; negative `N` reverses each window. Width zero uses identities at the `1+≢Y` boundaries of a vector.

```apl
2+/1 2 3 4         ⍝ 3 5 7
¯2-/1 2 3          ⍝ 1 1
0+/1 2             ⍝ 0 0 0
```

`[K]` selects the axis. [`⌿`](slash-first.md) defaults to the first. Float sum/product reductions allow reassociation; generic reductions retain operand order.
