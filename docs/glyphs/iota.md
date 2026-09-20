# `⍳` — Index generator / Index of

`⍳N` generates `1…N`. Dimensions are nonnegative integers; exact input gives exact coordinates.

```apl
⍳4                 ⍝ 1 2 3 4
⍳3x                ⍝ 1x 2x 3x
```

A shape vector generates an array of coordinate vectors.

```apl
⍳2 2               ⍝ 2 2⍴(1 1⋄ 1 2⋄ 2 1⋄ 2 2)
```

`X⍳Y` finds the first matching major cell in `X` for each cell of `Y`. Not found: `1+≢X`. Uses tolerant matching and returns exact positions.

```apl
'abc'⍳'cabz'        ⍝ 3x 1x 2x 4x
'abc'⍳'b'           ⍝ 2x
'abc'⍳⊂'b'          ⍝ 2x
```

A single query returns an atom. Batch axes supply the result shape. Enclose an array-valued query to search for it as one item.

```apl
C←'cat' 'dog'
C⍳⊂'dog'           ⍝ 2x
C⍳,⊂'dog'          ⍝ ,2x
m←3 2⍴10 20 30 40 50 60
m⍳30 40            ⍝ 2x
m⍳1 2⍴30 40        ⍝ ,2x
m⍳2 2⍴30 40 10 20  ⍝ 2x 1x
```
