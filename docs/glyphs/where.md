# `⍸` — Where / Interval index

`⍸Y` lists exact positions, repeated according to their nonnegative integer counts. Higher-rank arguments give coordinate vectors.

```apl
⍸0 2 0 1           ⍝ 2x 2x 4x
```

`X⍸Y` counts sorted boundaries in `X` ≤ each query. Uses [structural ordering](../rules.md#equality-and-ordering), without tolerance.

```apl
10 20 30⍸5 10 25 40 ⍝ 0x 1x 2x 3x
```
