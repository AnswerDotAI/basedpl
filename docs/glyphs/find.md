# `⍷` — Find

`X⍷Y` marks starting positions where the whole pattern `X` occurs in `Y`. The result has `Y`'s shape. Overlaps count.

```apl
'ana'⍷'banana'      ⍝ 0ₓ 1ₓ 0ₓ 1ₓ 0ₓ 0ₓ
1 1⍷1 1 1          ⍝ 1ₓ 1ₓ 0ₓ
```

Works with multidimensional patterns and tolerant element matching.
