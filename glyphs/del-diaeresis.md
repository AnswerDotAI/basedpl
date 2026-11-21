

# `⍢` — Operator recursion

Argument. Keys: `Alt-1 g`. Hanzi: `循`

Inside an operator [dfn](../dfns.ipynb), `⍢` is the operator itself.
Apply `⍢` to operands to derive the recursive function; `∇` keeps the
current operands.

``` bpl
op←{ω=0?0;≥⍶⍢≤ω} ◇ +op 3 ⍝ 3
```
