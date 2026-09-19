# `∨` — OR / GCD

`∨Y` gives real/imaginary pairs along a new last axis. Y is a numeric array.

```apl
∨3j4              ⍝ 3 4
∨3j4 1j2          ⍝ 2 2⍴3 4 1 2
∨1r3              ⍝ 1r3 0x
```

`X∨Y` gives GCD; on Booleans, OR. Pervasive.

```apl
0 0 1 1∨0 1 0 1   ⍝ 0 1 1 1
12x∨18x            ⍝ 6x
∨/0 0 1            ⍝ 1
```

Includes rational and complex GCD. Inputs are finite. Empty reduction identity: zero.
