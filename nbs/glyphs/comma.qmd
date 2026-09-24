# `,` — Ravel / Catenate

`,Y` makes a vector of the items. `,[K]Y` ravels consecutive axes; `,[]Y` appends a unit axis.

```apl
,2 2⍴⍳4            ⍝ 1 2 3 4
⍴,7                ⍝ ,1ₓ
```

`X,Y` joins on the last axis. `X,[K]Y` joins on axis `K`. Other axes agree, with scalar and adjacent-rank extension.

```apl
1 2,3 4            ⍝ 1 2 3 4
```

Fractional axes laminate: insert a new axis.

```apl
⍴1 2,[0.5]3 4      ⍝ 2ₓ 2ₓ
```
