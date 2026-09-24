# `⍤` — Atop / Rank

With function operand `g`, `f⍤g Y` is `f(g Y)` and `X f⍤g Y` is `f(X g Y)`.

```apl
2 -⍤+ 3            ⍝ ¯5
```

With numeric operand `R`, `f⍤R` applies to trailing cells of rank `R`. Frames use leading agreement; results assemble with fill.

```apl
(+/⍤1)2 3⍴⍳6       ⍝ 6 15
```

| `R` | Monad rank | Dyad left/right ranks |
|---|---|---|
| `r` | `r` | `r r` |
| `q r` | `r` | `q r` |
| `p q r` | `p` | `q r` |

Negative ranks count back from argument rank; excessive ranks clamp. Empty frames call the operand on prototype cells.
