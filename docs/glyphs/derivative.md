# `∂` — Derivative

`f∂X`: gradient; requires scalar output. `U(f∂)X`: vector–Jacobian product, with `U` shaped like the output. Result has `X`'s shape.

Currently supports real evaluators `C∘𝒫`, including factored and exponent-table forms.

```apl
f←1x 2x 3x∘𝒫 ⋄ f∂2x              ⍝ 14x
f←1x 2x 3x∘𝒫 ⋄ f∂∂2x             ⍝ 6x
f←1x 2x 3x∘𝒫 ⋄ 10x 20x(f∂)1x 2x  ⍝ 80x 280x
```

Multivariate gradients retain the coordinate enclosure. A shared scalar coordinate sums partials.

```apl
f←(⊂2 3⍴1x 2x 0x 1x 0x 2x)∘𝒫 ⋄ f∂⊂3x 4x  ⍝ ⊂6x 8x
f←(⊂2 3⍴1x 2x 0x 1x 0x 2x)∘𝒫 ⋄ f∂3x      ⍝ 12x
```

Repeated `∂` requires scalar input/output and one variable. DOMAIN: unsupported function or nonfinite/non-real data. RANK: monadic array output. LENGTH: wrong cotangent shape.
