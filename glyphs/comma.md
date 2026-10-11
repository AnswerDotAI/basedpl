

# `,` — Ravel / Catenate

Ranks: `∞` monadic, `∞ ∞` dyadic

`,Y` makes a vector of the items. With [axes](axis.qmd), `,⍠K Y` ravels
consecutive axes and `,⍠⍬ Y` appends an axis of length 1.

``` bpl
,[1 2⋄3 4]       ⍝ 1 2 3 4
⍴,7                ⍝ [1]ₓ
⍴,⍠1 2 (2 3 4⍴⍳24) ⍝ [2 12]ₓ
```

`X,Y` joins on the last axis. `X,⍠K Y` joins on axis `K`. Other axes
agree, with unit and adjacent-rank extension.

``` bpl
1 2,3 4            ⍝ 1 2 3 4
```

There is no laminate. `[x⋄y]` has `x` and `y` as its major cells, along
a new leading axis. `x,⍤0 y` pairs the items of `x` and `y` along a new
last axis.

``` bpl
x←1 2 ⋄ y←3 4
[x⋄y]            ⍝ [1 2⋄3 4]
x,⍤0 y             ⍝ [1 3⋄2 4]
```

## Inverse

`,⁻¹Y` gives back `Y` when it’s a vector. `X,⁻¹Y` removes the fixed `X`
from the front of `Y`. With the right argument fixed, `(,↢X)⁻¹Y` removes
it from the end.

``` bpl
,⁻¹1 2 3           ⍝ 1 2 3
1 2,⁻¹1 2 3 4      ⍝ 3 4
(,↢3 4)⁻¹1 2 3 4   ⍝ 1 2
```
