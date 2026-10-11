

# `⍟` — Natural log / Logarithm

Keys: `Alt-o 8`. Ranks: `0` monadic, `0 0` dyadic

`⍟Y`: ln Y. Pervasive.

``` bpl
⍟1                ⍝ 0
```

`X⍟Y`: log<sub>X</sub> Y. Pervasive.

``` bpl
2⍟8               ⍝ 3
10⍟100            ⍝ 2
```

Uses complex logarithms where needed. `⍟0` is `¯∞`, as IEEE 754 gives
it. With base 1, the result is an infinity or NaN.

## Inverse

`⍟⁻¹` is `*`. With the right argument fixed, `(⍟↢X)⁻¹Y` is `X*÷Y`.

``` bpl
2⍟⁻¹3      ⍝ 8
(⍟↢8)⁻¹3   ⍝ 2
```
