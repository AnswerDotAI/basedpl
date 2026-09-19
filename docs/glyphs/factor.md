# `𝒬` — Factor

`𝒬N`: sorted prime factors, with multiplicity. J's `q:` family. Positive integral input; exact integer results.

```apl
𝒬700                 ⍝ 2x 2x 5x 5x 7x
𝒬1                   ⍝ 0⍴0x
𝒬⍣¯1⊢𝒬700           ⍝ 700x
```

Inverse: product. `K𝒬N` selects exponents or a factor table.

| K | Result |
|---|---|
| Nonnegative integer | Exponents of the first K primes; remaining factors are omitted |
| `∞` | Exponents through the largest factor |
| Negative integer | Last `\|K` distinct factors and their exponents, in two rows |
| `¯∞` | Complete two-row factor/exponent table |

```apl
2𝒬700                ⍝ 2x 0x
∞𝒬700                ⍝ 2x 0x 2x 1x
¯2𝒬700               ⍝ 2 2⍴5x 7x 2x 1x
¯∞𝒬700               ⍝ 2 3⍴2x 5x 7x 2x 2x 1x
0𝒬700                ⍝ 0⍴0x
¯∞𝒬1                 ⍝ 2 0⍴0x
```

Scalar cells; results assemble with fill. Large factors follow [Prime's](prime.md) probable-prime policy.

DOMAIN: nonpositive/non-integral `N`; non-integral finite `K`.
