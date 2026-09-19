# `\` — Expand / Scan

`N\Y` expands the last axis. Positive counts consume and repeat an item; zero inserts one fill; negative counts insert `|N|` fills.

```apl
1 0 1\10 20        ⍝ 10 0 20
2 ¯2 1\10 20       ⍝ 10 10 0 0 20
```

`f\Y` scans left-to-right: each result becomes the next call's left argument.

```apl
+\1 2 3            ⍝ 1 3 6
-\1 2 3            ⍝ 1 ¯1 ¯4
```

`S f\Y` starts with seed `S`, excluding the seed from the result.

```apl
10 -\1 2 3         ⍝ 9 7 4
100 +\10 20        ⍝ 110 130
```

The whole seed starts each lane. Use Rank for separate seeds per cell.

```apl
10 20 (+\⍤0 1)2 3⍴⍳6  ⍝ 2 3⍴11 13 16 24 29 35
(,10)+\1 2 3           ⍝ (,11)(,13)(,16)
```

Empty scans and unseeded singletons make no calls. `[K]` selects the axis; [`⍀`](backslash-bar.md) defaults to the first.

APL difference: every scan left-accumulates, including dfns.
