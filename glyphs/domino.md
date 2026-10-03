

# `⌹` — Matrix inverse / Matrix divide

Keys: `Alt-q u`

`⌹Y` gives an inverse, or a full-column-rank least-squares
pseudoinverse. Units reciprocate.

``` bpl
⌹2ₓ                ⍝ 1r2
⌹[1 0 ⋄ 0 2]ₓ     ⍝ [1ₓ 0ₓ ⋄ 0ₓ 1r2]
```

`X⌹Y` solves `Y+.×Z = X`, using least squares when overdetermined.

``` bpl
[3 5 7]ₓ⌹[1 1 ⋄ 1 2 ⋄ 1 3]ₓ      ⍝ [1 2]ₓ
```

All-exact inputs use rational elimination. Approximate real/complex
inputs use pivoted LU for square systems and SVD for rectangular least
squares.

## Errors

- `DOMAIN`: singular or underdetermined system, non-finite input
