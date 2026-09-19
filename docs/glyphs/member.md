# `∊` — Enlist / Membership

`∊Y` recursively ravels all nested leaves.

```apl
∊(1 2)(3 (4 5))    ⍝ 1 2 3 4 5
```

`X∊Y` marks items of `X` found among the elements of `Y`, using tolerant matching. A single query returns an atom. Batch axes follow `X`'s shape.

```apl
1 2 3∊2 4          ⍝ 0x 1x 0x
'banana'∊'an'       ⍝ 0x 1x 1x 1x 1x 1x
C←'cat' 'dog'
(⊂'dog')∊C         ⍝ 1x
(,⊂'dog')∊C        ⍝ ,1x
```
