

# `•float` — Float width

`16 •float Y`, `32 •float Y` and `64 •float Y` give the numbers of `Y`
as floats of that many bits. Each number rounds to the nearest float of
the width. A complex number’s parts round to the width, or to 32 bits
for 16. Arrays nested in `Y` convert too. Under
[`∂`](../glyphs/partial.qmd), a tangent converts with its primal. A
gradient keeps its argument’s width, whatever width the function
computes at. [Float widths](../numbers.qmd#float-widths) gives the rules
that floats of each width follow.

``` bpl
v←32 •float 0.1 0.2 0.3
(•storage v).kind              ⍝ "float32"
(•storage 64 •float v).kind    ⍝ "float64"
```

`•float Y` gives the width of `Y`’s floats, as a number that `•float`’s
left argument takes. It’s the widest float width among the numbers in
`Y`, nested arrays included. When they’re all exact, it’s the default
width, which the `float` setting of [`•prefs`](prefs.qmd) chooses.
Arithmetic between 16-bit data and a computed value, which is
[strong](../numbers.qmd#weak-and-strong-numbers), gives the wider of
their widths. `(•float x)•float c` converts a computed `c` to the width
of the data `x`. A bare literal such as `0.5` is weak and takes the
data’s width without converting:

``` bpl
x←16 •float 1 2 3
•float x                                ⍝ 16ₓ
c←+/0.25 0.25
(•storage x×c).kind                     ⍝ "float64"
(•storage x×(•float x)•float c).kind    ⍝ "float16"
(•storage x×0.5).kind                   ⍝ "float16"
```

Errors: `DOMAIN` for a width other than 16, 32 or 64, and for characters
and functions.
