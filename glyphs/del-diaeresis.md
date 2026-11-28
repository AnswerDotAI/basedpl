

# [`⍢`](https://answerdotai.github.io/basedpl/glyphs/del-diaeresis.html) — Operator recursion

Argument. Keys: `Alt-1 g`. Hanzi: `循`

Inside an operator [dfn](../dfns.ipynb),
[`⍢`](https://answerdotai.github.io/basedpl/glyphs/del-diaeresis.html)
is the operator itself. Apply
[`⍢`](https://answerdotai.github.io/basedpl/glyphs/del-diaeresis.html)
to operands to derive the recursive function;
[`∇`](https://answerdotai.github.io/basedpl/glyphs/del.html) keeps the
current operands.

``` bpl
op←{ω=0?0;≥⍶⍢≤ω} ◇ +op 3 ⍝ 3
```
