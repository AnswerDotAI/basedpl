# `⍢` — Operator self-reference

Inside a dop, `⍢` denotes the operator itself. Apply operands again to derive the recursive function; `∇` instead retains the current operands.

```apl
op←{⍵=0:0 ⋄ 1+(⍶ ⍢)⍵-1} ⋄ (+op)3 ⍝ 3
```
