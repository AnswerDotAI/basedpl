

# `⌝` — Outer product

`X g⌝Y` applies `g` to every pair. Two atoms call `g` directly. Array
arguments supply the result shape `(⍴X),⍴Y`.

``` apl
5+⌝5               ⍝ 10
5+⌝⊂5              ⍝ ⊂10
1,⌝2               ⍝ 1 2
1 2+⌝10 20         ⍝ [11 21 ⋄ 12 22]
⍳3 =⌝ ⍳3           ⍝ [1ₓ 0ₓ 0ₓ ⋄ 0ₓ 1ₓ 0ₓ ⋄ 0ₓ 0ₓ 1ₓ]
```

Array frames collect pair results as items. See [inner
product](dot.qmd).
