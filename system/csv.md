

# `•csv` — CSV

`•csv text` parses CSV into a vector of columns. Headers become keys.
Numeric columns become numbers. An integer outside the 64-bit range is
read as its nearest float. Missing numeric cells become the `fill`
option’s value, NaN by default. Missing text cells become `""`.

`ifill` replaces missing cells in integer columns without replacing NaNs
in float columns. It must be a whole real number. It applies to inferred
integer columns and to integer columns requested by `types`, not to
Boolean or float columns.

`fills` overrides individual columns, as a record keyed by column name
or a vector with one value per column. Its values must be numeric for
numeric columns and text for text columns. The precedence is `fills`,
then `ifill` for integer columns, then `fill` for numeric columns or
`""` for text.

A fill is converted to its column’s inferred type when that type can
hold its value exactly. With `ifill` set to `0`, a missing integer is
read as `0ₓ`. NaN or `0.5` from `fill` or `fills` instead makes an
inferred integer column floating-point. A column whose cells are all
missing is text unless `types` gives it a numeric type.

Under [static kinds](../numbers.qmd#static-kinds), every integer column
is read at the `int` width whatever its values, as every new integer is.
A value that doesn’t fit raises a `DOMAIN` error that names the column.

The `types` option sets each column’s type, given as a value of that
type. A number, such as `0ₓ₈`, `0ₚ` or `$f`, makes a numeric column of
its type. That type must hold each of the column’s values exactly, and
the fill at its missing cells, or `•csv` raises a `DOMAIN` error. With a
float type, values are rounded to its width. With text, such as `""`,
the column is read as written: `02134` stays `"02134"`. A record sets
the columns that it names. A vector sets every column in order, and
needs one value for each column. A file with no header needs a vector:

``` bpl
•prefs ["kinds":"static"]
t←"id,price,flag,zip",(•ucs 10),"3000000000,1.5,1,02134",(•ucs 10),"7,2.25,0,10001"
T←["types":["id":0ₓ₈ "price":0ₚ "flag":$f "zip":""]] •csv t
T.id                       ⍝ [3000000000 7]ₓ₈
T.flag                     ⍝ $t $f
(•storage T.price).kind    ⍝ "float64"
T.zip                      ⍝ "02134" "10001"
•prefs ["kinds":"dynamic"]
["header":$f "types":["" 0ₓ]] •csv "a,12",(•ucs 10),"b,7"   ⍝ [["a" "b"] [12 7]ₓ]
```

`•csv⁻ T` writes CSV text for a vector of columns. Keys supply the
header. Column lengths must agree. NaN and the selected fill for each
column are written as empty cells. `fills`, `ifill` and `fill` have the
same precedence as on import.

`•csv` and `•csv⁻` both take options on the left: `header`, `separator`,
`quote`, `doublequote`, `escapechar`, `decimal`, `thousands`, `trim`,
`fill`, `ifill` and `fills`. `•csv` also takes `missing` and `types`,
and `•csv⁻` takes `forcequotes` and `eol`. `"forcequotes":2` quotes
every field. [CSV](../data.ipynb#csv) describes the options.

Errors: `DOMAIN` for invalid options, duplicate headers, incompatible
fills, and cells that CSV can’t hold: rationals that aren’t whole,
complex numbers, functions or nested cells, and for a value that doesn’t
keep itself in its column’s type; `LENGTH` for unequal record widths or
columns, or a `types` or `fills` vector of the wrong length.
