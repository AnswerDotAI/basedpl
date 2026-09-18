# `○` — Pi times / Circular functions

`○Y`: πY. Pervasive.

```apl
○0                ⍝ 0
```

`K○Y` selects a circular function. Pervasive; angles in radians.

```apl
1○0               ⍝ 0
2○0               ⍝ 1
9 11○3j4          ⍝ 3 4
10○3j4            ⍝ 5
```

| `k` | `k○y` | `(-k)○y` |
|---|---|---|
| 1 | sin | arcsin |
| 2 | cos | arccos |
| 3 | tan | arctan |
| 4 | `√(1+y²)` | APL branch of `√(y²−1)` |
| 5 | sinh | arcsinh |
| 6 | cosh | arccosh |
| 7 | tanh | arctanh |
| 8 | `√(-1−y²)` | negative of code 8 |
| 9 | real part | identity |
| 10 | magnitude | conjugate |
| 11 | imaginary part | multiply by `0j1` |
| 12 | phase | `exp(0j1×y)` |

Code 0 is `√(1−y²)`. Uses complex continuation where needed.
