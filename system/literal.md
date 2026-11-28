

# `•literal` — Literals

`•literal⁻ Y` writes `Y` as canonical BPL source text, always in APL
notation regardless of `•prefs`. `•literal text` reads such text back
into its value without running code. The text may hold literals and the
functions that `•literal⁻` writes:
[`ρ`](https://answerdotai.github.io/basedpl/glyphs/rho.html),
[`⊂`](https://answerdotai.github.io/basedpl/glyphs/left-shoe.html),
[`,`](https://answerdotai.github.io/basedpl/glyphs/comma.html),
[`:`](https://answerdotai.github.io/basedpl/glyphs/colon.html) and
`•ucs`. Nested, keyed and exact values are read back unchanged. Floats
are written with the marker of their width, `ₚ`, `ₛ` or `ₕ`, so that the
text is read back the same at any default width. `•hash •literal⁻ Y`
hashes any array.

``` bpl
•literal⁻ 2 3ρι6              ⍝ "[0 1 2◇3 4 5]ₚ"
•literal "[1 [2 3ₓ] ""ab""]"   ⍝ [1 [2 3ₓ] "ab"]
```

Errors: `DOMAIN` when the argument of `•literal⁻` holds a function or an
operator, and when the text given to `•literal` holds a name, another
function or more than one value; `SYNTAX` for text that doesn’t parse.
