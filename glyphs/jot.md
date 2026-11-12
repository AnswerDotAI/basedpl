

# `∘` — Atop

Dyadic operator. Keys: `Alt-t`. Hanzi: `承`

`f∘g Y` is `f(g Y)`, and `X f∘g Y` is `f(X g Y)`.

``` bpl
2 -∘+ 3            ⍝ ¯5
```

See [Rank](jot-diaeresis.qmd) for applying a function to cells of a
chosen rank.

## Inverse

`(f∘g)⁻Y` is `g⁻ f⁻ Y`. `X(f∘g)⁻Y` is `X g⁻ f⁻ Y`.

``` bpl
(-∘÷)⁻¯0.5   ⍝ 2
```
