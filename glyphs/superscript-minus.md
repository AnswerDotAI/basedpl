

# `⁻` — Inverse

Monadic operator. Keys: `Alt-_`. Hanzi: `反`

`f⁻` is `f⍣¯1`. `f⁻Y` gives the `X` for which `f X` is `Y`. In a dyadic
call, `X f⁻Y` keeps `X` fixed and finds the right argument `Z` for which
`X f Z` is `Y`.

``` bpl
*⁻1           ⍝ 0
2×⁻10         ⍝ 5
+↢3 ⁻ 10      ⍝ 7
```

BPL infers inverses of the primitives and compositions listed under
[Power](star-diaeresis.qmd). `f⁻←g` [attaches an
inverse](../assignment.ipynb#inverses-and-derivatives) to a named
function.

The minus is bare: no superscript digit follows it. `f⁻¹` means
[Rank](jot-diaeresis.qmd) ¯1. `f⁻ ¹` applies the inverse at rank 1.
Numeric superscripts on arrays remain powers, including the reciprocal
`2⁻¹`.

``` bpl
⌽⁻ ¹[1 2◇3 4]   ⍝ [2 1◇4 3]
```

An inverse applies to the whole derived function on its left. A space
groups a function that ends in a number: `+↢3 ⁻ 10`. Parentheses also
work: `(+↢3)⁻10`.

Errors: `DOMAIN` for an unknown inverse or an array operand.
