

# `{ }` — Dfn / Dop

`{…}` defines a function of
[`α`](https://answerdotai.github.io/basedpl/glyphs/alpha.html) and
[`ω`](https://answerdotai.github.io/basedpl/glyphs/omega.html), or an
operator when it uses
[`⍶`](https://answerdotai.github.io/basedpl/glyphs/alpha-underbar.html)
or
[`⍹`](https://answerdotai.github.io/basedpl/glyphs/omega-underbar.html).
`;` separates its bodies. When a [predicate](question.qmd) `C?` is
false, evaluation moves on to the next body. The last statement gives
the result. See [Dfns](../dfns.ipynb).

``` bpl
{ω×2}3             ⍝ 6
2{α+ω}3            ⍝ 5
{ω<0?-ω;ω}¯3       ⍝ 3
```
