

# `√` — Sqrt / Root

Function. Keys: `Alt-3`. Ranks: `0` monadic, `0 0` dyadic

`√Y`: square root. Pervasive. Every result is approximate, for exact
arguments too.

``` bpl
√9 ¯4             ⍝ 3 0j2
√1r4              ⍝ 0.5
```

`X√Y`: principal Xth root. Negative real Y with odd integral X uses the
real root. A negative degree gives the reciprocal of the root. `¯2√0` is
`∞`.

``` bpl
3√¯8              ⍝ ¯2
¯3ₓ√¯8ₓ           ⍝ ¯0.5
2√3j4             ⍝ 2j1
```

## Inverse

`√⁻¹Y` squares `Y`. `X√⁻¹Y` is `Y*X`. With the right argument fixed,
`(√↢X)⁻¹Y` is `Y⍟X`.

``` bpl
√⁻¹3       ⍝ 9
3√⁻¹2      ⍝ 8
(√↢8)⁻¹2   ⍝ 3
```

## Errors

- `DOMAIN`: zero or infinite degree
