# `⊤` — Encode

`B⊤Y` represents numbers in bases `B`. A zero base retains the remaining quotient.

```apl
2 2 2 2⊤10         ⍝ 1 0 1 0
0x 60x 60x⊤3661x   ⍝ 1x 1x 1x
```

Scalar `B` gives residues. Array bases add leading result axes; exact operands retain exact arithmetic.

```apl
10⊤12 34           ⍝ 2 4
```

See [Decode](decode.md).
