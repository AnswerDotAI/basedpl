# `⌹` — Matrix inverse / Matrix divide

`⌹Y` gives an inverse, or a full-column-rank least-squares pseudoinverse. Scalars reciprocate.

```apl
⌹2x                ⍝ 1r2
⌹2 2⍴1x 0x 0x 2x  ⍝ 2 2⍴1x 0x 0x 1r2
```

`X⌹Y` solves `Y+.×Z = X`, using least squares when overdetermined.

```apl
3x 5x 7x⌹3 2⍴1x 1x 1x 2x 1x 3x ⍝ 1x 2x
```

All-exact inputs use rational elimination. Approximate real/complex inputs use pivoted LU for square systems and SVD for rectangular least squares.

DOMAIN: singular or underdetermined system, non-finite input.
