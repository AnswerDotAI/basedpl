# `⍣\` — History

`f⍣\N Y`: states `0…N`, including `Y`. Negative `N` steps through the inverse. Dyadic calls hold `X` fixed.

```apl
(1∘+)⍣\3⊢5      ⍝ 5 6 7 8
(1∘+)⍣\¯2⊢5     ⍝ 5 4 3
{1÷0}⍣\0⊢'ab'   ⍝ 1 2⍴'ab'
```

`f⍣\g Y` stops when `new g old` is true. Includes initial and terminating states; always takes at least one step.

```apl
1 +⍣\{⍺=3}0     ⍝ 0 1 2 3
⊢⍣\≡⊢4          ⍝ 4 4
{⍵,1}⍣\2⊢,2     ⍝ 3 3⍴2 0 0 2 1 0 2 1 1
n←{0.5×⍵+2÷⍵} ⋄ ≢n⍣\≡⊢1  ⍝ 7x
```

States assemble with fill. `⍣ \` is the same operator; `(f⍣p)\` is ordinary Scan.

RANK: nonscalar count; use [Power](power.md). DOMAIN: non-integral count, non-Boolean predicate, unknown inverse.
