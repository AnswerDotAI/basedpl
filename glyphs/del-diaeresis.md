

# `⍢` — Operator self-reference

Keys: `Alt-1 g`

Inside a dop, `⍢` denotes the operator itself. Apply operands again to
derive the recursive function; `∇` instead retains the current operands.

``` bpl
op←{⍵=0?0;≥⍶⍢≤⍵} ⋄ +op 3 ⍝ 3
```
