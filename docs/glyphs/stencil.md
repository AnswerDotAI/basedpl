# `⌺` — Stencil

`f⌺S Y` applies `f` to neighbourhoods of sizes `S` on leading axes. Windows receive prototype fill at edges.

```apl
{+/,⍵}⌺3⊢1 2 3 4   ⍝ 3 6 9 7
```

`⍵` is the window; `⍺` gives signed padding counts per axis: positive at the start, negative at the end. A two-row specification gives sizes then movements.

```apl
{+/,⍵}⌺(2 1⍴3 2)⍳8 ⍝ 3 9 15 21
```

Results assemble with fill. DOMAIN: nonpositive sizes/movements, window too large for an axis.
