

# `,` — Ravel / Catenate

`,Y` makes a vector of the items. With [axes](axis.qmd), `,⍠K Y` ravels
consecutive axes and `,⍠⍬ Y` appends a unit axis.

``` apl
,[1 2 ⋄ 3 4]       ⍝ 1 2 3 4
⍴,7                ⍝ ,1ₓ
⍴,⍠1 2 (2 3 4⍴⍳24) ⍝ 2ₓ 12ₓ
```

`X,Y` joins on the last axis. `X,⍠K Y` joins on axis `K`. Other axes
agree, with scalar and adjacent-rank extension.

``` apl
1 2,3 4            ⍝ 1 2 3 4
```

There is no laminate. Array notation joins arrays along a new leading
axis, and Rank pairs their items along a new last axis.

``` apl
x←1 2 ⋄ y←3 4
[x ⋄ y]            ⍝ [1 2 ⋄ 3 4]
x,⍤0 y             ⍝ [1 3 ⋄ 2 4]
```
