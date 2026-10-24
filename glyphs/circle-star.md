

# `⍟` — Natural log / Logarithm

Function. Keys: `Alt-o 8`. Hanzi: `对` (`對`). Ranks: `0` monadic, `0 0`
dyadic

`⍟Y`: ln Y. Pervasive.

``` bpl
⍟1                ⍝ 0
```

`X⍟Y`: log<sub>X</sub> Y. Pervasive.

``` bpl
2⍟8               ⍝ 3
10⍟100            ⍝ 2
1⍟1               ⍝ 0
```

Uses complex logarithms where needed. `⍟0` is `¯∞`. `1⍟1` gives zero
when its floating calculation has a [Mathish
type](../terms.qmd#term-mathish-types) and [Mathish
mode](../numbers.qmd#mathish-mode) is enabled. Other base-1 cases give
an infinity or NaN.

## Inverse

`⍟⁻¹` is `*`. With the right argument fixed, `(⍟↢X)⁻¹Y` is `X*÷Y`.

``` bpl
2⍟⁻¹3      ⍝ 8
(⍟↢8)⁻¹3   ⍝ 2
```
