

# Axis keys

[Glyph index](glyphs.qmd)

An array can have string keys on any of its axes. Keys name positions on
that axis. Numeric coordinates count from 0. Array operations retain
their ordinary meaning.

## Construction

One string names the whole right value. Several strings name its
leading-axis cells.

``` apl
T←"price" "qty":[[1 2 3] [4 5 6]]
≢T                          ⍝ 2ₓ
⍴"ab":11 12 13              ⍝ ,1ₓ
"price" "qty":2 3⍴⍳6        ⍝ "price" "qty":⍠0 (2 3⍴⍳6)
≢⍬:⍬                        ⍝ 0ₓ
```

Brackets with any item written as `key:value` build one keyed vector.
Items without a key are positions with no key, so `["aa":1 2]` has a key
only on its first position. Keyed vectors display in this form. A vector
that holds a record needs brackets round the record: `[15 ["aa":1]]` has
two items, and the second is a record.

``` apl
T←["price":[1 2 3] "qty":[4 5 6]]
T≡"price" "qty":[[1 2 3] [4 5 6]]   ⍝ 1ₓ
["kind":"bar"]≡"kind":"bar"       ⍝ 1ₓ
```

A character atom or string is one string key. Characters side by side
form one string, so `'a' 'b'` is the key `"ab"`. Use `"a" "b"` for two
one-character keys.

``` apl
⍳⍠0 ('a' 'b':1 2)           ⍝ ["ab";]
⍳⍠0 ("a" "b":1 2)           ⍝ "a" "b"
```

`K:⍠axis M` attaches keys to an existing axis. Store the result with
`←`.

``` apl
M←2 3⍴⍳6
M←"Jan" "Feb" "Mar":⍠1 M
⍳⍠1 M                       ⍝ "Jan" "Feb" "Mar"
∞ "Feb"⌷M                   ⍝ 1 4
```

Several key vectors label successive axes. `:⍠` with several axes
chooses their axes in the supplied order. Brackets with `;` write the
key vectors, because each one contains spaces.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
⍴M                          ⍝ 2ₓ 2ₓ
⍳⍠0 M                       ⍝ "alice" "bob"
⍳⍠1 M                       ⍝ "price" "qty"
⍳⍠0 1 M                     ⍝ [["alice" "bob"] ["price" "qty"]]
M≡["price" "qty";"alice" "bob"]:⍠1 0 [10 2 ⋄ 20 4] ⍝ 1ₓ
```

Monadic `:` removes all keys. `:⍠` with axes removes only the selected
axes’ keys.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
:M                          ⍝ [10 2 ⋄ 20 4]
:⍠1 M                       ⍝ "alice" "bob":[10 2 ⋄ 20 4]
⍳⍠0 1 (:M)                  ⍝ [[0ₓ 1ₓ] [0ₓ 1ₓ]]
```

Inside dfns, group functional colon: `(X:Y)` or `(:Y)`. A direct colon
separates a guard.

``` apl
{+/(:⍵)}"aa" "bb":1 2       ⍝ 3
{("total":+/⍵)}1 2 3        ⍝ "total":6
```

Keys must be unique strings on each axis: `DOMAIN`. A key-list length
must match its axis: `LENGTH`.

<table>
<thead>
<tr>
<th>Expression</th>
<th>Error</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"aa" "aa":1 2</code></td>
<td><code>DOMAIN</code></td>
</tr>
<tr>
<td><code>"aa" "bb":⍠0 [1 2 3]</code></td>
<td><code>LENGTH</code></td>
</tr>
</tbody>
</table>

## Selection and assignment

[Index](glyphs/squad.qmd) `I⌷M` takes one selector for each leading
axis. Dot indexing, `M.(I)`, writes the same selection after the array.
An array applied to a selector selects on its leading axis, so `M "bob"`
is `"bob"⌷M`. A selector is a key or a position. A scalar selector
removes its axis. A vector selector retains it. A character vector
selects one string key.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
"bob"⌷M                     ⍝ "price" "qty":20 4
M "bob"                     ⍝ "price" "qty":20 4
1⌷M                         ⍝ "price" "qty":20 4
∞ "qty"⌷M                   ⍝ "alice" "bob":2 4
"alice" "qty"⌷M             ⍝ 2
"bob" 0⌷M                   ⍝ 20
[,⊂"alice";]⌷M              ⍝ [["alice";] ["price" "qty"]]:1 2⍴10 2
("bob" "qty")⊃M             ⍝ 4
M⍎"bob"                     ⍝ "price" "qty":20 4
```

Dot names are literal keys. `T.a.b` is `"b"⊃"a"⊃T`.

``` apl
T←["n":1 "addr":["city":"LA"]]
T.addr.city                 ⍝ "LA"
T.n+←1 ⋄ T.n                ⍝ 2
```

Plain assignment through a dot path adds missing keys. Missing names
along the path become records. On an axis with no keys, the new position
gets a key and the existing positions have none: after
`v←10 20 30 ⋄ v.x←9`, `v` is `[10 20 30 "x":9]`.

``` apl
T←["n":1]
T.style.color←"red"
T.style                     ⍝ ["color":"red"]
```

Plain assignment through dot indexing appends missing rows or columns,
as it does through `⌷`. New matrix cells receive the array’s prototype
before assignment.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
M.("cara")←30 5
"cara" "qty"⌷M                ⍝ 5
M.(∞ "tax")←1 2 3
"alice" "tax"⌷M               ⍝ 1
```

Appending on several axes fills the new cross-cells too.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
M.("cara" "tax")←3
"cara"⌷M                    ⍝ "price" "qty" "tax":0 0 3
∞ "tax"⌷M                   ⍝ "alice" "bob" "cara":0 0 3
```

Missing keys on read or modified assignment: `INDEX`. Repeated
selections on a keyed axis: `DOMAIN`. Numeric out-of-range assignment:
`INDEX`.

For `T←"aa" "bb":1 2`:

<table>
<thead>
<tr>
<th>Expression</th>
<th>Error</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"cc"⌷T</code></td>
<td><code>INDEX</code></td>
</tr>
<tr>
<td><code>T.("cc")+←1</code></td>
<td><code>INDEX</code></td>
</tr>
<tr>
<td><code>[0 0;]⌷T</code></td>
<td><code>DOMAIN</code></td>
</tr>
<tr>
<td><code>T.(2)←9</code></td>
<td><code>INDEX</code></td>
</tr>
</tbody>
</table>

Scalar selection stores the whole RHS. A vector selection aligns its
retained axis.

``` apl
T←"aa" "bb":1 2 ⋄ U←"bb" "aa":8 9
T.("aa")←U ⋄ "aa"⌷T           ⍝ "bb" "aa":8 9
T.[,⊂"aa";]←U ⋄ "aa"⌷T        ⍝ 9
```

## Agreement

Corresponding keyed axes align by union: left keys, then right-only
keys. An absent counterpart receives the present value’s prototype.

``` apl
("aa":2)+("bb":3)            ⍝ "aa" "bb":2 3
("aa":2)×("bb":3)            ⍝ "aa" "bb":0 0
("aa":0)=("bb":0)            ⍝ "aa" "bb":1ₓ 1ₓ
```

Each keyed axis aligns independently, including reordered keys.

``` apl
A←[["alice";] ["price";]]:1 1⍴10
B←[["bob";] ["qty";]]:1 1⍴2
A+B                         ⍝ ["alice" "bob";"price" "qty"]:[10 0 ⋄ 0 2]
("aa" "bb":1 2)+("bb" "aa":10 20) ⍝ "aa" "bb":21 12
```

With one axis unkeyed, agreement is positional. Expanding a keyed
singleton drops that axis’s labels. Enclosure broadcasts within the
named value.

``` apl
("base":5)+10 20 30          ⍝ 15 25 35
("base":5)+⊂10 20 30         ⍝ "base":15 25 35
("aa" "bb":1 2)+10 20        ⍝ "aa" "bb":11 22
```

An axis operand chooses the correspondence.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
taxed←M+⍠1 ("qty" "tax":10 2)
taxed   ⍝ ["alice" "bob";"price" "qty" "tax"]:[10 12 2 ⋄ 20 14 2]
```

## Array operations

Transpose moves axes with their keys.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
⍉M                          ⍝ ["price" "qty";"alice" "bob"]:[10 20 ⋄ 2 4]
```

Reverse, rotate, take, drop and filtering move keys with their
positions.

``` apl
V←"aa" "bb" "cc":10 20 5
⌽V                          ⍝ "cc" "bb" "aa":5 20 10
1⌽V                         ⍝ "bb" "cc" "aa":20 5 10
¯1↑V                        ⍝ "cc":5
1↓V                         ⍝ "bb" "cc":20 5
1 0 1/V                     ⍝ "aa" "cc":10 5
```

Reduction removes the reduced axis. Scan retains it. Each retains its
frame keys.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
+/M                         ⍝ "alice" "bob":12 24
+⌿M                         ⍝ "price" "qty":30 6
+\"aa" "bb" "cc":1 2 3       ⍝ "aa" "bb" "cc":1 3 6
+/¨["price":[1 2 3] "qty":[4 5 6]] ⍝ ["price":6 "qty":15]
```

Shape-changing reshape drops keys. Same-shape reshape retains them.
Ravel and table drop keys on merged/split axes, retaining unchanged
axes.

``` apl
V←"aa" "bb":1 2
2⍴V                         ⍝ "aa" "bb":1 2
4⍴V                         ⍝ 1 2 1 2
M←"aa" "bb":2 2⍴⍳4
,M                          ⍝ 0 1 2 3
⍳⍠0 ⍪M                      ⍝ "aa" "bb"
```

Rank preserves frame keys. Assembled cell axes keep keys shared by every
result cell in the same order.

``` apl
V←"aa" "bb":1 2
{[⍵ ⍵]}⍤0 V                 ⍝ "aa" "bb":[1 1 ⋄ 2 2]
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
⌽⍤1 M                       ⍝ ["alice" "bob";"qty" "price"]:[2 10 ⋄ 4 20]
{10=↑⍵:⌽⍵ ⋄ ⍵}⍤1 M          ⍝ "alice" "bob":[2 10 ⋄ 20 4]
```

Per-cell rotation drops keys on the rotated axis when cells use
different rotations.

``` apl
M←["alice" "bob";"price" "qty"]:[10 2 ⋄ 20 4]
0 1⌽M                       ⍝ "alice" "bob":[10 2 ⋄ 4 20]
```

Outer product retains both arguments’ axes. Array-valued power counts
supply the result frame.

``` apl
("aa" "bb":1 2)+⌝("xx" "yy":10 20) ⍝ ["aa" "bb";"xx" "yy"]:[11 21 ⋄ 12 22]
N←"initial" "once" "twice":0 1 2
2×⍣N 3                      ⍝ "initial" "once" "twice":3 6 12
```

Inner product, Decode and matrix solve align contracted keys. Both
keyed: equal key sets required (`LENGTH` otherwise). Either unkeyed:
positional contraction.

``` apl
("hi" "lo":1 2)+.×("lo" "hi":10 20) ⍝ 40
("hi" "lo":10 10)⊥("lo" "hi":2 1) ⍝ 12
M←["r1" "r2";"xx" "yy"]:[2x 0x ⋄ 0x 4x]
("r2" "r1":12x 4x)⌹M        ⍝ "xx" "yy":2ₓ 3ₓ
⌹M                          ⍝ ["xx" "yy";"r1" "r2"]:[1r2 0ₓ ⋄ 0ₓ 1r4]
```

Catenate joins keys on its joined axis and aligns other keyed axes.
Positions from an unkeyed part of the joined axis have no keys.

``` apl
("aa":1),("bb":2)            ⍝ "aa" "bb":1 2
("aa" "bb":1 2),5            ⍝ ["aa":1 "bb":2 5]
A←["aa" "bb";"xx" "yy"]:[1 2 ⋄ 3 4]
B←["bb" "aa";["zz";]]:[5 ⋄ 6]
A,B                         ⍝ ["aa" "bb";"xx" "yy" "zz"]:[1 2 6 ⋄ 3 4 5]
```

Replicate can repeat an unkeyed axis while retaining keys on other axes.

``` apl
2/"aa" "bb":[1 2 ⋄ 3 4]     ⍝ "aa" "bb":[1 1 2 2 ⋄ 3 3 4 4]
```

Take and Expand add positions with no key, as fills from an unkeyed part
do.

``` apl
V←"aa" "bb":1 2
3↑V                         ⍝ ["aa":1 "bb":2 0]
1 0 1\V                     ⍝ ["aa":1 0 "bb":2]
```

Repeated positions on a keyed axis give `DOMAIN`. For `V←"aa" "bb":1 2`:

<table>
<thead>
<tr>
<th>Expression</th>
<th>Reason</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>V,V</code></td>
<td>Duplicate joined keys</td>
</tr>
<tr>
<td><code>2/V</code></td>
<td>Repeated keys</td>
</tr>
</tbody>
</table>

### Windows

Full windows have an unkeyed position axis. Each window retains its
slice’s keys. Assembly keeps only keys shared by all windows. Padded
windows and stencil frames retain their centre keys. Padding drops keys
on the padded window axis.

``` apl
V←"aa" "bb" "cc":1 2 3
2↕V                         ⍝ [1 2 ⋄ 2 3]
3↕V                         ⍝ "aa" "bb" "cc":⍠1 [1 2 3 ⋄]
¯3↕V                        ⍝ "aa" "bb" "cc":[0 1 2 ⋄ 1 2 3 ⋄ 2 3 0]
+/⍤⊢⌺3 V                    ⍝ "aa" "bb" "cc":3 6 5
{(⍳⍠0 ⍵)≡"aa" "bb" "cc"}⌺3 V ⍝ "aa" "bb" "cc":0ₓ 1ₓ 0ₓ
```

## Match and search

Match aligns keys independently on every keyed axis. Shapes and unkeyed
axes must match positionally. Searches compare values or cells. Found
positions return axis keys.

``` apl
V←"low" "mid" "high":10 20 30
V≡⌽V                        ⍝ 1ₓ
V⍳20 99                     ⍝ "mid" 3ₓ
V⍸5 15 25 40                ⍝ 0ₓ "low" "mid" "high"
⍒V                          ⍝ "high" "mid" "low"
V ⍒V                        ⍝ "high" "mid" "low":30 20 10
("aa":7)∊("bb":7)            ⍝ "aa":1ₓ
```

Multidimensional positions mix keys and numeric coordinates. Bare Iota
generates from values as usual.

``` apl
M←"price" "qty":⍠1 [10 2 ⋄ 20 4]
↑⍸M=20                      ⍝ 1ₓ "price"
⍳"rows" "cols":2 3          ⍝ ⍳2 3
```

Unique and set operations compare values and retain the selected
positions’ keys.

``` apl
A←"aa" "bb":1 2 ⋄ B←"xx" "yy":2 3
A∩B                         ⍝ "bb":2
A~B                         ⍝ "aa":1
A∪B                         ⍝ "aa" "bb" "yy":1 2 3
∪"aa" "bb" "cc":1 1 2       ⍝ "aa" "cc":1 2
```

Monadic Key passes named positions to its operand. Dyadic Key preserves
labels on the grouped values.

``` apl
V←"aa" "bb" "cc":1 2 1
{⊂⍵}⌸V                     ⍝ [["aa" "cc"] ["bb";]]
1 2 1{⊂⍵}⌸V                ⍝ [["aa":1 "cc":1] ["bb":2]]
```

## Named axes

An axis name identifies a dimension: `city`. Its keys identify
positions: `LA NY`. Names are unique within an array.

An array’s shape is keyed by its axis names. Reshape by a keyed shape to
name the axes.

``` apl
M←["city":2 "month":3]⍴⍳6
⍴M                            ⍝ ["city":2ₓ "month":3ₓ]
⍴M "month"                    ⍝ 3ₓ
+/⍠"month" M                  ⍝ ["city":2]⍴3 12
```

An unkeyed entry of the shape leaves its axis unnamed. `(N:⍴M)⍴M`
renames the axes, and `(:⍴M)⍴M` removes the names. In `"city" 1`, the
integer key equals its own position, counting from 0, so that position
has no key. A reshape that keeps the shape also keeps the position keys
and shares the element buffer. Assign with `M←…` to store the result.

``` apl
M←["city":2 "month":3]⍴⍳6
⍴("city" 1:⍴M)⍴M              ⍝ ["city":2ₓ 3ₓ]
⍴(:⍴M)⍴M                      ⍝ 2ₓ 3ₓ
```

A keyed vector on the left of `:` supplies both axis names and position
keys. Its keys name axes. Its values list their positions.

``` apl
sales←["city":["NY" "LA"] "month":["Jan" "Feb" "Mar"]]:[10 20 30 ⋄ 40 50 60]
⍴sales                        ⍝ ["city":2ₓ "month":3ₓ]
⍳⍠0 1 sales                   ⍝ [["NY" "LA"] ["Jan" "Feb" "Mar"]]
"LA" "Feb"⌷sales              ⍝ 50
```

Reuse descriptions, or attach them to selected axes. Other axes keep
their names and keys.

``` apl
months←"month":"Jan" "Feb" "Mar"
M←months:⍠1 (2 3⍴⍳6)
⍴M                            ⍝ [2ₓ "month":3ₓ]
⍳⍠1 M                         ⍝ "Jan" "Feb" "Mar"
⍴("city":"NY" "LA"):M         ⍝ ["city":2ₓ "month":3ₓ]
```

``` apl
axes←["month":["Jan" "Feb" "Mar"] "city":["NY" "LA"]]
M←axes:⍠1 0 (2 3⍴⍳6)
⍴M                            ⍝ ["city":2ₓ "month":3ₓ]
⍳⍠0 1 M                       ⍝ [["NY" "LA"] ["Jan" "Feb" "Mar"]]
```

Agreement first pairs equal names, wherever they occur. Unpaired axes
broadcast. Result order is left axes, then right-only axes.

``` apl
M←["city":2 "month":3]⍴⍳6
N←["month":3]⍴10 20 30
M+N                           ⍝ ["city":2 "month":3]⍴[10 21 32 ⋄ 13 24 35]
⍴N+M                          ⍝ ["month":3ₓ "city":2ₓ]
M+⍉M                          ⍝ ["city":2 "month":3]⍴[0 2 4 ⋄ 6 8 10]
```

Remaining axes pair in leading-axis order. A named/unnamed pair keeps
the name. Two different names remain separate broadcast dimensions.
Length agreement and position-key alignment then apply.

``` apl
M←["city":2 "month":3]⍴⍳6
M+10 20                       ⍝ ["city":2 "month":3]⍴[10 11 12 ⋄ 23 24 25]
A←["city":2 3]⍴1
B←["product":4 3]⍴2
⍴A+B                          ⍝ ["city":2ₓ 3ₓ "product":4ₓ]
```

A string or a list of strings as the `⍠` operand names axes. Index `⌷`
with that operand selects positions on the named axes.

``` apl
M←["city":["LA" "NY"] "month":["Jan" "Feb" "Mar"]]:2 3⍴⍳6
⍳⍠0 ("LA"⌷⍠"city" M)          ⍝ "Jan" "Feb" "Mar"
("LA"⌷M)≡"LA"⌷⍠"city" M       ⍝ 1ₓ
⍴⊂⍠"month" M                  ⍝ ["city":2ₓ]
+/⍠"city" "month" M          ⍝ 15
```

Transpose moves names. Scalar selection removes the selected axis.
Repeating positions retains names. Reshape takes its names from its left
argument. Axis merging drops names on replaced axes. New axes start
unnamed.

``` apl
M←["city":2 "month":3]⍴⍳6
⍴⍉M                          ⍝ ["month":3ₓ "city":2ₓ]
⍴0⌷M                         ⍝ ["month":3ₓ]
⍴2 3⍴M                       ⍝ 2ₓ 3ₓ
⍴,M                          ⍝ ,6ₓ
V←["city":2]⍴1 2
⍴2/V                         ⍝ ["city":4ₓ]
⍴⍪V                          ⍝ ["city":2ₓ 1ₓ]
```

Duplicating an axis name drops every occurrence of it. Position keys
remain.

``` apl
V←["city":2]⍴"LA" "NY":1 2
W←V×⌝V
⍴W                           ⍝ 2ₓ 2ₓ
⍳⍠0 1 W                      ⍝ [["LA" "NY"] ["LA" "NY"]]
```

<table>
<thead>
<tr>
<th>Expression</th>
<th>Error</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>+/⍠"missing" M</code></td>
<td><code>INDEX</code></td>
</tr>
<tr>
<td><code>["city":2 "city":3]⍴0</code></td>
<td><code>DOMAIN</code></td>
</tr>
</tbody>
</table>

## Python and JSON

Python dicts become keyed vectors recursively. `.py` converts them back
to dicts.

``` python
from basedpl import Array
import numpy as np

record = Array({'price': 10, 'address': {'city': 'LA'}})
assert record.py == {'price': 10, 'address': {'city': 'LA'}}
assert record.axis_keys == (('price', 'address'),)
```

Attach position keys with `axis_keys`, axis names with `axis_names`.
`None` leaves an axis unkeyed/unnamed. The corresponding properties
inspect them; `.np` copies the values to NumPy.

``` python
rows, cols = ['NY', 'LA'], ['Jan', 'Feb', 'Mar']
data = [[10, 20, 30], [40, 50, 60]]
sales = Array(data, axis_keys=(rows, cols), axis_names=('city', 'month'))
assert sales.axis_keys == (tuple(rows), tuple(cols))
assert sales.axis_names == ('city', 'month')
assert sales.np.tolist() == data
columns_only = Array(data, axis_keys=(None, cols), axis_names=(None, 'month'))
assert columns_only.axis_keys == (None, tuple(cols))
assert columns_only.axis_names == (None, 'month')
```

`.df` converts to pandas. For a matrix, keys label rows/columns and axis
names name the index/columns. `.py` uses this conversion for keyed
arrays of rank ≥2. Install pandas with `pip install pandas`; it is
imported on conversion.

``` python
df = sales.df
assert df.loc['LA', 'Feb'] == 50
assert (df.index.name, df.columns.name) == ('city', 'month')
assert sales.py.equals(df)
assert columns_only.df.index.tolist() == [0, 1]
```

Above rank 2, the last axis supplies columns; earlier axes form a row
MultiIndex. Unkeyed axes use labels that count from 0. Scalars and
vectors become one-column DataFrames.

``` python
counts = np.arange(12).reshape(2, 2, 3)
names = 'year', 'city', 'month'
cube = Array(counts, axis_keys=(None, rows, cols), axis_names=names)
assert cube.df.index.names == ['year', 'city']
assert cube.df.loc[(1, 'LA'), 'Mar'] == 11
assert Array({'aa': 1, 'bb': 2}).df.loc['bb', 0] == 2
assert Array(7).df.loc[0, 0] == 7
```

[`•json`](data.ipynb#json) converts ordinary JSON objects to keyed
vectors. The [process protocol](processes.qmd) carries `axis_keys` and
`axis_names` beside shape, row-major data and prototype. For `sales`
above:

``` json
{
  "shape": [2, 3],
  "data": [10, 20, 30, 40, 50, 60],
  "prototype": 0,
  "axis_keys": [["NY", "LA"], ["Jan", "Feb", "Mar"]],
  "axis_names": ["city", "month"]
}
```

Use `null` for an unkeyed/unnamed axis. Omit each metadata field when
none of its axes has that metadata:

``` json
{"shape":[1,2],"data":[10,20],"prototype":0,"axis_keys":[null,["Jan","Feb"]]}
{"shape":[2],"data":[10,20],"prototype":0}
```
