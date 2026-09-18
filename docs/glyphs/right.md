# `⊢` — Right

`⊢Y` and `X⊢Y` return `Y`.

```apl
⊢1 2               ⍝ 1 2
1 2⊢3 4            ⍝ 3 4
```

Both arguments evaluate. `⊢` also separates a derived function from its argument.

```apl
(+∘1)⍣3⊢0          ⍝ 3
```
