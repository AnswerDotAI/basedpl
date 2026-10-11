

# `•json` — JSON

`•json text` reads JSON5, which includes all JSON. JSON5 adds comments,
trailing commas, single-quoted strings, unquoted keys, hexadecimal
numbers, `Infinity` and `NaN`. Objects become keyed vectors. Arrays
become vectors. Strings become character vectors. Integers of up to 128
bits stay exact. `true` and `false` become `$t` and `$f`.

`•json⁻¹ Y` writes JSON text. Keyed axes become objects. Unkeyed axes
become arrays. Character vectors become strings. Keyed entries that hold
functions are left out. NaN writes as `null`.

`["fill":v] •json text` reads `null` as `v`. The default is NaN. An
integer array with a `null` stays exact. `["fill":v] •json⁻¹ Y` writes
`v` as `null`. [JSON](../data.ipynb#json) covers the conversions.

Errors: `DOMAIN` for malformed JSON, with its line and column, and for
values that JSON can’t hold: an infinity that isn’t `fill`, out-of-range
floats, nonintegral rationals, complex numbers and other functions.
