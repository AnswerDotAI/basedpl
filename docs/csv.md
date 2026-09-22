# CSV

`•CSV text` reads CSV into a vector of column vectors. Headers become keys. Each numeric column uses compact integer or float storage where possible.

```apl
nl←•UCS 10
text←'price,qty',nl,'10.5,2',nl,'20.0,4'
T←•CSV text
T                         ⍝ 'price' 'qty':(10.5 20 ⋄ 2x 4x)
'qty'⊃T                   ⍝ 2x 4x
+/¨T                      ⍝ 'price' 'qty':30.5 6x
```

`T •CSV ''` writes CSV text. Column lengths must agree. Keys supply the header. Unkeyed input writes data alone.

```apl
T←'price' 'qty':(10.5 20 ⋄ 2x 4x)
•CSV T •CSV ''            ⍝ 'price' 'qty':(10.5 20 ⋄ 2x 4x)
```

## Options

Pass a keyed vector. Import includes `source`; export takes the table on the left. Option names are case-insensitive.

```apl
nl←•UCS 10
text←'price;qty',nl,'10,5;2',nl,'20,0;4'
T←•CSV 'source' 'separator' 'decimal':(text ⋄ ';' ⋄ ',')
T                         ⍝ 'price' 'qty':(10.5 20 ⋄ 2x 4x)
csv←T •CSV 'separator' 'decimal':(';' ⋄ ',')
•CSV 'source' 'separator' 'decimal':(csv ⋄ ';' ⋄ ',')
```

| Option | Default | Meaning |
|---|---|---|
| `source` | Required for import | CSV text |
| `header` | Import: `1`; export: has keys | Read/write column names |
| `separator` | `','` | Field separator; use `•UCS 9` for TSV |
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
| `lineending` | `•UCS 10` | Export: LF or CRLF (`•UCS 13 10`) |

Column selectors are names or 1-origin positions. A character vector names one column. Use a vector for several selectors.

```apl
text←'id,qty',(•UCS 10),'00123,5'
T←•CSV 'source' 'text_columns':(text ⋄ 'id')
'id'⊃T                    ⍝ ,⊂'00123'
```

Separator, quote and escape must be distinct ASCII characters other than CR, LF or NUL. Fields support Unicode, doubled quotes and embedded newlines. Import accepts LF, CRLF and CR record endings. Export with `escapechar` quotes every field and doubles literal escape characters. With quoting disabled, export errors if a field requires quoting.

## Numbers and missing cells

Inference examines a whole column, ignoring missing cells. Integer fields produce exact numbers. Decimal/scientific notation produces floats. A column containing other text stays text. Quoted numbers participate in inference too.

```apl
nl←•UCS 10
T←•CSV 'count,label',nl,'2,001',nl,'3,yes'
'count'⊃T                 ⍝ 2x 3x
'label'⊃T                 ⍝ '001' 'yes'
```

Integer-only columns use `i64` storage when values fit. Decimal columns promote integers to floats only when every conversion is exact. Other numeric columns retain mixed storage. Integers beyond `i64` remain exact.

Missing numeric cells become `∞`. Missing text cells become `''`. Entirely missing columns are text unless forced numeric.

```apl
nl←•UCS 10
T←•CSV 'price,qty',nl,'10.5,2',nl,',4'
'price'⊃T                 ⍝ 10.5 ∞
'qty'⊃T                   ⍝ 2x 4x
```

Configure extra missing markers and numeric fill explicitly.

```apl
text←'qty',(•UCS 10),'NA',(•UCS 10),'4'
•CSV 'source' 'missing' 'fill':(text ⋄ 'NA' ⋄ ¯1x)   ⍝ 'qty':¯1x 4x
```

Export uses `-` for negative numbers, decimal/scientific float spelling and plain exact integers. Infinity writes as `inf` or `-inf`. Set `fill` to export a chosen numeric sentinel as an empty field. Nonintegral rationals, complex numbers, functions and nested nonstring cells give DOMAIN.

Empty input returns an empty unkeyed vector. Header-only input returns keyed empty columns. Empty header names are allowed; duplicate names give DOMAIN. Unequal record widths or column lengths give LENGTH. Invalid forced numeric fields report their row and column.

## Python and files

```python
from basedpl import Session

with Session() as apl:
    csv = apl.fn('•CSV')
    table = csv('price,qty\n10.5,2\n20.0,4\n')
    assert table.py['qty'].tolist() == [2, 4]
    text = csv(table, {'separator': ';'}).py
    restored = csv({'source': text, 'separator': ';'})
    assert restored.py['price'].tolist() == [10.5, 20.0]
```

File access stays separate. With `pathlib.Path`, read using `csv(path.read_text(encoding='utf-8'))`. Write using `path.write_text(csv(table, '').py, encoding='utf-8')`.
