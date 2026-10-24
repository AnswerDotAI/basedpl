

# `•csv` — CSV

`•csv text` parses CSV into a vector of columns. Headers become keys.
Numeric columns become numbers. Missing numeric cells become NaN. An
integer column with missing cells stays exact. Missing text cells become
`""`.

`•csv⁻¹ T` writes CSV text for a vector of columns. Keys supply the
header. Column lengths must agree. NaN writes as an empty cell, and so
does the exact value given as `fill`.

Both directions take options on the left: `header`, `separator`,
`quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`, `trim`
and `fill`. Reading also takes `text_columns`, `numeric_columns` and
`missing`, and writing takes `forcequotes` and `lineending`.
`"forcequotes":2` quotes every field. [CSV](../data.ipynb#csv) describes
the options.

Errors: `DOMAIN` for invalid options, duplicate headers, and cells that
CSV can’t hold: nonintegral rationals, complex numbers, functions or
nested cells; `LENGTH` for unequal record widths or columns.
