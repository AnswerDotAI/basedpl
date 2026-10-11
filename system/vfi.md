

# `•vfi` — Numeric input

`•vfi text` returns two vectors, `[valid numbers]`, for the
whitespace-separated fields of `text`. An invalid field has the flag
`0ₓ` and the value `0`. Fields are parsed as numbers, never executed.

`separators •vfi text` splits on each character in `separators` instead.
It trims whitespace around fields. An empty field is a valid `0`.
[Numeric input](../data.ipynb#numeric-input) covers it.

Errors: `DOMAIN` when either argument is not text.
