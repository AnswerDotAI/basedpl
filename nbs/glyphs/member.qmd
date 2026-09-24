# `∊` — Enlist / Membership

`∊Y` recursively ravels all nested leaves.

```apl
∊(1 2⋄ 3 (4 5))    ⍝ 1 2 3 4 5
```

`X∊Y` marks items of `X` found among the elements of `Y`, using tolerant matching. A single query returns an atom. Batch axes follow `X`'s shape.

```apl
1 2 3∊2 4          ⍝ 0ₓ 1ₓ 0ₓ
'banana'∊'an'       ⍝ 0ₓ 1ₓ 1ₓ 1ₓ 1ₓ 1ₓ
C←'cat' 'dog'
(⊂'dog')∊C         ⍝ 1ₓ
(,⊂'dog')∊C        ⍝ ,1ₓ
```
