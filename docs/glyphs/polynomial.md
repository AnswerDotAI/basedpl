# `Ⓟ` — Polynomial

J's `p.` family. Ranks: `1` monadic, `1 0` dyadic.

## Coefficients

`CⓅX`: evaluate **constant-first** coefficients at `X`. [Decode](decode.md) uses the opposite order.

```apl
1x 2x 3xⓅ0x 1x 2x   ⍝ 1x 6x 17x
```

`ⓅC`: leading coefficient and numerical roots, as a nested pair. Ignores trailing zeros. Constants have no roots.

```apl
≢2⊃Ⓟ1 0 1           ⍝ 2x
```

## Multiplier and roots

`(M R)ⓅX`: `M××/X-R`. `⊂R` assumes `M=1`; use `⊂,R` for one root. Monadic `Ⓟ` gives coefficients.

```apl
(2x (1x 3x))Ⓟ0x 1x 2x 3x  ⍝ 6x 0x ¯2x 0x
Ⓟ2x (1x 3x)                ⍝ 6x ¯8x 2x
Ⓟ⊂1x 3x                    ⍝ 3x ¯4x 1x
```

## Coefficient/exponent tables

Enclosed matrix: one row per term, coefficient then exponents. Enclose one coordinate vector to evaluate one point. A single coordinate extends to all variables. Batch axes collect answers.

```apl
(⊂2 3⍴1x 2x 0x 1x 0x 2x)Ⓟ⊂3x 4x  ⍝ 25x
(⊂2 3⍴1x 2x 0x 1x 0x 2x)Ⓟ(3x 4x)(5x 12x)  ⍝ 25x 169x
(⊂2 2⍴2x 1r2 3x 1r4)Ⓟ16x           ⍝ 14
Ⓟ⊂2 2⍴1x 5x ¯1x 0x                 ⍝ ¯1x 0x 0x 0x 0x 1x
```

Evaluation accepts fractional/negative exponents. Conversion to coefficients requires one variable and nonnegative integral exponents; repeated degrees add.

Derivative coefficients: `(1↓C)×⍳¯1+≢C`. Integral: `K,C÷⍳≢C`. See [Derivative](derivative.md).

LENGTH: wrong coordinate count. DOMAIN: nonnumeric data, invalid conversion exponents or failed root solve.
