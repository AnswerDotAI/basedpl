# Files, CSV and JSON

Read text with `•nget`, parse it with `•csv` or `•json`, calculate, then serialize and write with `•nput`.

```text
sales←•csv •nget 'sales.csv'
totals←+/¨sales
(totals •json '') •nput 'totals.json'
```

CSV, JSON and file functions accept keyed options. Option names are case-insensitive.

## Files

`•nget path` reads UTF-8 text, preserving newlines. `text •nput path` creates a UTF-8 file and returns the number of bytes written. Writing an existing path gives VALUE unless `overwrite` is `1`.

```text
text←•nget 'sales.csv'
text •nput 'copy.csv'
text •nput ('path' 'overwrite':('copy.csv' ⋄ 1))
```

| Option | Default | Meaning |
|---|---|---|
| `path` | Required | Filename; a plain path is shorthand |
| `encoding` | `'UTF-8'` | UTF-8 |
| `binary` | `0` | Read/write numeric byte vectors instead of text; omit `encoding` |
| `overwrite` | `0` | `•nput`: replace an existing file |

Missing files, invalid UTF-8 and OS file errors give VALUE. Invalid options give DOMAIN. Directories must already exist.

Binary reads return exact integers in `0…255`. Binary writes accept a vector of integral numbers in that range. Validation precedes opening the output file. The result is the byte count.

```text
bytes←•nget 'path' 'binary':('image.bin' ⋄ 1)
bytes •nput 'path' 'binary':('copy.bin' ⋄ 1)
'UTF-8' •ucs bytes           ⍝ decode UTF-8 when appropriate
bytes-256x×bytes≥128x         ⍝ signed-byte interpretation
```

Byte vectors use ordinary compact integer storage. RANK: output is not a vector. DOMAIN: invalid byte, or `binary:1` with `encoding`.

## Numeric input

`•vfi text` returns `(valid ⋄ numbers)`. Whitespace separates fields. Invalid fields have flag `0x` and value `0`; valid fields have flag `1x`. Input is parsed as numbers, never executed.

```apl
valid nums←•vfi '12 nope -3 1.5'
valid                      ⍝ 1x 0x 1x 1x
nums                       ⍝ 12 0 ¯3 1.5
valid/nums                 ⍝ 12 ¯3 1.5
```

Numbers retain their literal domains: ordinary spelling is approximate; `x` and `r` are exact. Complex `j`/`J`, infinity, ASCII signs and signed exponents are accepted. `inf`/`infinity` also denote infinity. NaN and invalid numbers produce flag `0x`.

```apl
2⊃•vfi '2 3x 1r4 1j-2 -1e-3 ∞'   ⍝ 2 3x 1r4 1j¯2 ¯0.001 ∞
```

`separators •vfi text` splits on any supplied character and trims surrounding whitespace. Empty fields are valid zero. Internal whitespace remains part of the field.

```apl
',' •vfi '3.9,2.4,,76,'   ⍝ ((5⍴1x) ⋄ 3.9 2.4 0 76 0)
'⋄' •vfi '1 ⋄ 2 3 ⋄ 4'   ⍝ (1x 0x 1x ⋄ 1 0 4)
```

Empty input returns two empty vectors. With no separator characters (`''` on the left), nonempty input is one field. DOMAIN: either argument is not character text.

## JSON

`•json text` parses JSON. Objects become keyed vectors; arrays become vectors. Strings become character vectors. Nested structure is retained.

```apl
person←•json '{"name":"Ann","scores":[10,20]}'
'name'⊃person              ⍝ 'Ann'
'scores'⊃person            ⍝ 10x 20x
```

`Y •json ''` exports. Keyed axes form objects; unkeyed axes form arrays. Character vectors form strings, and scalar arrays export their contents. Axis names and empty-array prototypes are omitted.

```apl
('name' 'scores':('Ann' ⋄ 10x 20x)) •json ''
[1x 2x ⋄ 3x 4x] •json ''   ⍝ '[[1,2],[3,4]]'
(•json '{}') •json ''      ⍝ '{}'
```

Integer tokens remain exact, including large integers. Decimal/exponent tokens become floats. `true` and `false` become `1x` and `0x`, which export as numbers.

```apl
•json '[9223372036854775808,1.5,true,false]'
(•json '[true,false]') •json ''   ⍝ '[1,0]'
```

JSON `null` becomes `∞` by default. Set `fill` to choose another numeric sentinel. On export, an explicitly supplied `fill` becomes `null` wherever it occurs.

```apl
•json '[1,null,3]'                         ⍝ 1x ∞ 3x
•json 'source' 'fill':('[1,null,3]' ⋄ ¯1x)   ⍝ 1x ¯1x 3x
(1x ∞ 3x) •json 'fill':∞                    ⍝ '[1,null,3]'
```

| Option | Default | Meaning |
|---|---|---|
| `source` | Required for import | JSON text; plain text is shorthand |
| `fill` | Import: `∞`; export: none | Numeric replacement for `null` |

Malformed JSON gives DOMAIN with line/column details. Out-of-range floats, nonintegral rational exports, complex exports and functions give DOMAIN. Infinity requires explicit export fill. Duplicate object members retain the last value.

## CSV

`•csv text` reads CSV into a vector of column vectors. Headers become keys. Each numeric column uses compact integer or float storage where possible.

```apl
nl←•ucs 10
text←'price,qty',nl,'10.5,2',nl,'20.0,4'
T←•csv text
T                         ⍝ 'price' 'qty':(10.5 20 ⋄ 2x 4x)
'qty'⊃T                   ⍝ 2x 4x
+/¨T                      ⍝ 'price' 'qty':30.5 6x
```

`T •csv ''` writes CSV text. Column lengths must agree. Keys supply the header. Unkeyed input writes data alone.

```apl
T←'price' 'qty':(10.5 20 ⋄ 2x 4x)
•csv T •csv ''            ⍝ 'price' 'qty':(10.5 20 ⋄ 2x 4x)
```

### CSV options

Pass a keyed vector. Import includes `source`; export takes the table on the left. Option names are case-insensitive.

```apl
nl←•ucs 10
text←'price;qty',nl,'10,5;2',nl,'20,0;4'
T←•csv 'source' 'separator' 'decimal':(text ⋄ ';' ⋄ ',')
T                         ⍝ 'price' 'qty':(10.5 20 ⋄ 2x 4x)
csv←T •csv 'separator' 'decimal':(';' ⋄ ',')
•csv 'source' 'separator' 'decimal':(csv ⋄ ';' ⋄ ',')
```

| Option | Default | Meaning |
|---|---|---|
| `source` | Required for import | CSV text |
| `header` | Import: `1`; export: has keys | Read/write column names |
| `separator` | `','` | Field separator; use `•ucs 9` for TSV |
| `quotechar` | `'"'` | Quote character; `''` disables quoting |
| `doublequote` | `1` | Represent a quote inside a quoted field by doubling it |
| `escapechar` | `''` | Escape character inside quoted fields |
| `decimal` | `'.'` | Decimal mark: `'.'` or `','` |
| `thousands` | `''` | Group separator; groups after the first contain three digits |
| `trim` | `0` | Trim surrounding whitespace from fields |
| `fill` | Import: `∞`; export: none | Numeric missing-cell replacement; on export, this exact value writes as an empty cell |
| `text_columns` | `⍬` | Import these columns as text |
| `numeric_columns` | `⍬` | Import these columns as numbers; invalid nonmissing text errors |
| `missing` | `⍬` | Additional missing-cell strings; empty cells are always missing |
| `forcequotes` | `0` | Export: `0` as needed, `2` all fields |
| `lineending` | `•ucs 10` | Export: LF or CRLF (`•ucs 13 10`) |

Column selectors are names or 1-origin positions. A character vector names one column. Use a vector for several selectors.

```apl
text←'id,qty',(•ucs 10),'00123,5'
T←•csv 'source' 'text_columns':(text ⋄ 'id')
'id'⊃T                    ⍝ ,⊂'00123'
```

Separator, quote and escape must be distinct ASCII characters other than CR, LF or NUL. Fields support Unicode, doubled quotes and embedded newlines. Import accepts LF, CRLF and CR record endings. Export with `escapechar` quotes every field and doubles literal escape characters. With quoting disabled, export errors if a field requires quoting.

### Numbers and missing cells

Inference examines a whole column, ignoring missing cells. Integer fields produce exact numbers. Decimal/scientific notation produces floats. A column containing other text stays text. Quoted numbers participate in inference too.

```apl
nl←•ucs 10
T←•csv 'count,label',nl,'2,001',nl,'3,yes'
'count'⊃T                 ⍝ 2x 3x
'label'⊃T                 ⍝ '001' 'yes'
```

Integer-only columns use `i64` storage when values fit. Decimal columns promote integers to floats only when every conversion is exact. Other numeric columns retain mixed storage. Integers beyond `i64` remain exact.

Missing numeric cells become `∞`. Missing text cells become `''`. Entirely missing columns are text unless forced numeric.

```apl
nl←•ucs 10
T←•csv 'price,qty',nl,'10.5,2',nl,',4'
'price'⊃T                 ⍝ 10.5 ∞
'qty'⊃T                   ⍝ 2x 4x
```

Configure extra missing markers and numeric fill explicitly.

```apl
text←'qty',(•ucs 10),'NA',(•ucs 10),'4'
•csv 'source' 'missing' 'fill':(text ⋄ 'NA' ⋄ ¯1x)   ⍝ 'qty':¯1x 4x
```

Export uses `-` for negative numbers, decimal/scientific float spelling and plain exact integers. Infinity writes as `inf` or `-inf`. Set `fill` to export a chosen numeric sentinel as an empty field. Nonintegral rationals, complex numbers, functions and nested nonstring cells give DOMAIN.

Empty input returns an empty unkeyed vector. Header-only input returns keyed empty columns. Empty header names are allowed; duplicate names give DOMAIN. Unequal record widths or column lengths give LENGTH. Invalid forced numeric fields report their row and column.

## Python and files

```python
from basedpl import Session

with Session() as apl:
    csv = apl.fn('•csv')
    table = csv('price,qty\n10.5,2\n20.0,4\n')
    assert table.py['qty'].tolist() == [2, 4]
    text = csv(table, {'separator': ';'}).py
    restored = csv({'source': text, 'separator': ';'})
    assert restored.py['price'].tolist() == [10.5, 20.0]
    json = apl.fn('•json')
    encoded = json(table, '').py
    assert json(encoded).py['qty'].tolist() == [2, 4]
```

Use `apl.fn('•nget')` and `apl.fn('•nput')` for the same file operations from Python. Python's `pathlib.Path.read_text` and `write_text` work with the codec functions too.
