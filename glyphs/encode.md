

# `⊤` — Encode

Keys: `Alt-n`

`⊤Y` encodes nonnegative integers in binary, choosing enough digits for
the largest value. Each number’s digits lie along a new last axis. Zero
needs zero digits.

``` bpl
⊤5                ⍝ 1 0 1
⊤[2 5]ₓ           ⍝ [0 1 0 ⋄ 1 0 1]ₓ
```

`B⊤Y` represents numbers in bases `B`. A zero base retains the remaining
quotient.

``` bpl
2 2 2 2⊤10         ⍝ 1 0 1 0
[0 60 60]ₓ⊤3661ₓ   ⍝ [1 1 1]ₓ
```

A unit `B` gives residues. A vector `B` adds a new last axis of digits.
The result’s shape is `(⍴Y),⍴B`. Exact operands retain exact arithmetic.

``` bpl
9⊤15               ⍝ 6
9⊤⊂15              ⍝ ⊂6
10⊤12 34           ⍝ 2 4
2 2 2⊤3 5          ⍝ [0 1 1 ⋄ 1 0 1]
```

See [Decode](decode.qmd).
