

# `⊖` — Reverse first / Rotate first

Function. Keys: `Alt-o Minus`. Hanzi: `纵` (`縱`). Ranks: `∞` monadic,
`∞ ∞` dyadic

First-axis forms of [`⌽`](circle-stile.qmd). `⊖⍠K` overrides the axis.

``` bpl
⊖2 3⍴⍳6ₓ            ⍝ [3 4 5⋄0 1 2]ₓ
1⊖3 2⍴⍳6ₓ           ⍝ [2 3⋄4 5⋄0 1]ₓ
```

## Inverse

`⊖⁻¹Y` is `⊖Y`. `X⊖⁻¹Y` rotates `Y` back by `X`.

``` bpl
1⊖⁻¹[3 4⋄1 2]   ⍝ [1 2⋄3 4]
```
