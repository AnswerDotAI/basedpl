

# `•float` — Float width

`16 •float Y`, `32 •float Y` and `64 •float Y` give the numbers of `Y`
as floats of that many bits. `•float Y` gives floats of the default
width, which the `float` setting of [`•prefs`](prefs.qmd) chooses. Each
number rounds to the nearest float of the width. Arrays nested in `Y`
convert too. [Float widths](../numbers.qmd#float-widths) gives the rules
that floats of each width follow.

``` bpl
v←32 •float 0.1 0.2 0.3
•storage v              ⍝ "float32"
•storage 64 •float v    ⍝ "float64"
```

Errors: `DOMAIN` for a width other than 16, 32 or 64, for characters and
functions, and for complex numbers below 64 bits, because their parts
are 64-bit.
