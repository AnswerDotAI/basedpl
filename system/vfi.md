

# `•vfi` — Numeric input

`•vfi text` returns two vectors, `[valid numbers]`, with a flag and a
value for each whitespace-separated field of `text`. An invalid field
has the flag `0ₓ` and the value zero. The zero is converted to the other
values’ type, as a reader’s `fill` is: `•vfi "2ₓ nope"` gives
`[[1 0]ₓ;[2 0]ₓ]`. Fields are parsed as numbers, never executed.

A field can be any of BPL’s written forms of a number, with plain
letters or subscripts. `•vfi` reads back the numbers that
[`⍕`](https://answerdotai.github.io/basedpl/glyphs/thorn.html) displays:

``` bpl
•vfi ⍕1r3 1j2 1e20   ⍝ [[1 1 1]ₓ [1ᵣ3 1ⱼ2 1ₑ20]]
```

`separators •vfi text` splits on each character in `separators` instead.
It trims whitespace around fields. An empty field is a valid `0`.
[Numeric input](../data.ipynb#numeric-input) covers it.

Errors: `DOMAIN` when either argument is not text.
