

# [`⊣`](https://answerdotai.github.io/basedpl/glyphs/left-tack.html) — Same left / Left

Function. Keys: `Alt-;`. Hanzi: `左`. Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `∞ ∞` dyadic

`⊣Y` returns `Y`.

``` bpl
⊣1 2               ⍝ 1 2
```

`X⊣Y` returns `X`. Both arguments are evaluated.

``` bpl
1 2⊣3 4            ⍝ 1 2
```

See [trains](parentheses.qmd).

## Inverse

`⊣⁻` is
[`⊣`](https://answerdotai.github.io/basedpl/glyphs/left-tack.html).
`X⊣⁻Y` is `Y`, which must match `X`.

``` bpl
3⊣⁻3   ⍝ 3
```
