# `ℙ` — Prime

`ℙN`: Nth prime, one-based. J's `p:` family. Integral inputs; exact integer results.

```apl
ℙ⍳8               ⍝ 2x 3x 5x 7x 11x 13x 17x 19x
ℙ⍣¯1⊢ℙ⍳4         ⍝ 1x 2x 3x 4x
```

`KℙN` selects an operation; codes retain J's values.

| K | Result |
|---|---|
| `¯4` | Previous prime, strictly below N |
| `¯1` | Number of primes strictly below N |
| `0` | N is not prime |
| `1` | N is prime |
| `2` | Two-row distinct-factor/exponent table |
| `3` | Repeated prime factors |
| `4` | Next prime, strictly above N |
| `5` | Euler's totient |

```apl
¯1ℙ1 2 3 4 5 6     ⍝ 0x 0x 1x 2x 2x 3x
1ℙ¯1 0 1 2 3 4     ⍝ 0x 0x 0x 1x 1x 0x
4ℙ1 2 3 4 5        ⍝ 2x 3x 5x 5x 7x
5ℙ1 2 3 4 5 6      ⍝ 1x 1x 2x 2x 4x 2x
2ℙ700               ⍝ 2 3⍴2x 5x 7x 2x 2x 1x
```

Scalar cells; results assemble with fill. Inverse: count-below plus one. Primality testing is deterministic through 64 bits, probabilistic above that (false-positive bound `2⁻⁶⁴`).

DOMAIN: invalid selector/input; previous prime at/below 2. See [Factor](factor.md).
