# `∊` — Enlist / Membership

`∊Y` recursively ravels all nested leaves.

```apl
∊(1 2)(3 (4 5))    ⍝ 1 2 3 4 5
```

`X∊Y` marks items of `X` found in `Y`. Preserves `X`'s shape and uses tolerant matching.

```apl
1 2 3∊2 4          ⍝ 0x 1x 0x
'banana'∊'an'       ⍝ 0x 1x 1x 1x 1x 1x
```
