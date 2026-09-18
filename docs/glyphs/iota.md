# `⍳` — Index generator / Index of

`⍳N` generates `1…N`. Dimensions are nonnegative integers; exact input gives exact coordinates.

```apl
⍳4                 ⍝ 1 2 3 4
⍳3x                ⍝ 1x 2x 3x
```

A shape vector generates an array of coordinate vectors.

```apl
⍳2 2               ⍝ 2 2⍴(1 1)(1 2)(2 1)(2 2)
```

`X⍳Y` finds the first matching major cell in `X` for each cell of `Y`. Not found: `1+≢X`. Uses tolerant matching and returns exact positions.

```apl
'abc'⍳'cabz'        ⍝ 3x 1x 2x 4x
```
