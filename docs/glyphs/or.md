# `∨` — OR / GCD

`X∨Y` gives GCD; on Booleans, OR. Pervasive.

```apl
0 0 1 1∨0 1 0 1   ⍝ 0 1 1 1
12x∨18x            ⍝ 6x
∨/0 0 1            ⍝ 1
```

Includes rational and complex GCD. Inputs are finite. Empty reduction identity: zero.
