# `⍷` — Find

`X⍷Y` marks starting positions where the whole pattern `X` occurs in `Y`. The result has `Y`'s shape. Overlaps count.

```apl
'ana'⍷'banana'      ⍝ 0x 1x 0x 1x 0x 0x
1 1⍷1 1 1          ⍝ 1x 1x 0x
```

Works with multidimensional patterns and tolerant element matching.
