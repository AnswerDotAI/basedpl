

# `-` — Negate / Subtract

`-Y` negates. Pervasive.

``` bpl
-3                ⍝ ¯3
```

`X-Y` subtracts. Pervasive.

``` bpl
10-1 2 3          ⍝ 9 8 7
```

Characters shift by code point. Character differences are exact
integers.

``` bpl
'd'-3             ⍝ 'a'
'd'-'a'           ⍝ 3ₓ
```

[`¯`](overbar.qmd) marks negative literals.
