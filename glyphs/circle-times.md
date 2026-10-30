

# `⊗` — Outer product

Monadic operator. Keys: `Alt-o m`. Hanzi: `表`

`X g⊗Y` applies `g` to every pair. Two atoms call `g` directly. Array
arguments supply the result shape `⍴X , ⍴Y`.

``` bpl
5+⊗5               ⍝ 10
5+⊗⊂5              ⍝ ⊂10
1,⊗2               ⍝ 1 2
1 2+⊗10 20         ⍝ [11 21⋄12 22]
⍳3ₓ =⊗ ⍳3ₓ           ⍝ [$t $f $f⋄$f $t $f⋄$f $f $t]
```

Array frames collect pair results as items. See [inner
product](dot.qmd).

Where it can, `⊗` calls `g` once on all the pairs together, in a
[batched call](../evaluation.qmd#functions-and-effects). The result is
the same as from one call for each pair.

## Inverse

With either argument fixed, `X f⊗⁻¹ Y` gives the argument whose outer
product with `X` is `Y`.

``` bpl
1 2+⊗⁻¹1 2+⊗10 20   ⍝ 10 20
```
