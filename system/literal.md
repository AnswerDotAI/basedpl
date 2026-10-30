

# `•literal` — Literals

`•literal⁻¹ Y` writes `Y` as BPL source text, in the notation that
display uses. `•literal text` reads such text back into its value
without running code. The text may hold literals and the functions that
`•literal⁻¹` writes: `⍴`, `⊂`, `,`, `:` and `•ucs`. Nested, keyed and
exact values read back unchanged. Floats carry the marker of their
width, `ₚ`, `ₛ` or `ₕ`, so the text reads back the same at any default
width. `•hash •literal⁻¹ Y` hashes any array.

``` bpl
•literal⁻¹ 2 3⍴⍳6              ⍝ "[0 1 2⋄3 4 5]ₚ"
•literal "[1 [2 3ₓ] ""ab""]"   ⍝ [1 [2 3ₓ] "ab"]
```

Errors: `DOMAIN` when the argument of `•literal⁻¹` holds a function or
an operator, and when the text given to `•literal` holds a name, another
function or more than one value; `SYNTAX` for text that doesn’t parse.
