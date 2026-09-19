# `⇄` — Inverse pair

`f⇄g` calls `f`; its inverse is `g⇄f`.

```apl
f←{⍵+1}⇄{⍵-1} ⋄ f 5           ⍝ 6
f←{⍵+1}⇄{⍵-1} ⋄ f⍣¯2⊢5       ⍝ 3
```

Dyadic inversion holds `X` fixed: `X g (X f Y)` must recover `Y`.

```apl
f←{⍺+⍵}⇄{⍵-⍺} ⋄ 3(f⍣¯1)8    ⍝ 5
f←{⍺+⍵}⇄{⍵-⍺} ⋄ (3∘f)⍣¯1⊢8 ⍝ 5
```

Left binding preserves the pair. For right binding, pair the bound monad: `(f∘A)⇄undo`.

See [Power](power.md) and [Under](under.md).
