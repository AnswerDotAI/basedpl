

# [`⍥`](https://answerdotai.github.io/basedpl/glyphs/circle-diaeresis.html) — Over

Dyadic operator. Keys: `Alt-1 o`. Hanzi: `经` (`經`)

`f⍥g Y` is `f(g Y)`.

``` bpl
+/⍥, [1 2◇3 4]   ⍝ 10
```

`X f⍥g Y` is `(g X)f(g Y)`.

``` bpl
1 2 +⍥≢ 3 4 5      ⍝ 5ₓ
```

Compare [After](left-tail-arrow.qmd), which transforms only the right
argument.

## Inverse

`X(f⍥g)⁻Y` is `g⁻((g X)f⁻Y)`.

``` bpl
1(+⍥⊽)⁻8   ⍝ 3
```
