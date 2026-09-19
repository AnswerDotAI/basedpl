# `⊥` — Decode

`⊥Y` decodes binary digits along the first axis.

```apl
⊥1 0 1 0          ⍝ 10
```

`B⊥Y` evaluates digits `Y` in base(s) `B`. Scalar base expands; the first mixed-base entry is unused.

```apl
2⊥1 0 1 0          ⍝ 10
0x 60x 60x⊥1x 1x 1x ⍝ 3661x
```

For coefficient vector `C`, `X⊥C` evaluates the polynomial in descending powers.

```apl
3⊥2 4 5            ⍝ 35
```

Arrays contract the last base axis with the first digit axis. See [Encode](encode.md).
