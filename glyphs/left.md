

# `⊣` — Left

Keys: `Alt-;`. Ranks: `∞` monadic, `∞ ∞` dyadic

`⊣Y` returns `Y`.

``` bpl
⊣1 2               ⍝ 1 2
```

`X⊣Y` returns `X`. Both arguments evaluate.

``` bpl
1 2⊣3 4            ⍝ 1 2
```

See [trains](parentheses.qmd).

## Inverse

`⊣⁻¹` is `⊣`. `X⊣⁻¹Y` is `Y`, which must match `X`.

``` bpl
3⊣⁻¹3   ⍝ 3
```
