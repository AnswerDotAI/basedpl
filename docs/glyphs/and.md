# `∧` — AND / LCM

`X∧Y` gives LCM; on Booleans, AND. Pervasive.

```apl
0 0 1 1∧0 1 0 1   ⍝ 0 0 0 1
4x∧6x              ⍝ 12x
∧/1 1 0            ⍝ 0
```

Includes rational and complex LCM. Inputs are finite. Empty reduction identity: one.
