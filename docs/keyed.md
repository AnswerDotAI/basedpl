# Axis keys

[Glyph index](index.md#glyph-reference)

An array can have string keys on any of its axes. Keys name positions on that axis. Numeric coordinates remain 1-origin. Array operations retain their ordinary meaning.

## Construction

One string names the whole right value. Several strings name its leading-axis cells.

```apl
T←'price' 'qty':(1 2 3 ⋄ 4 5 6)
≢T                          ⍝ 2x
⍴('ab':11 12 13)            ⍝ ,1x
'price' 'qty':2 3⍴⍳6       ⍝ ('price' 'qty':[1]2 3⍴⍳6)
≢⍬:⍬                        ⍝ 0x
```

A character atom or vector is one string key. Use `,¨'ab'` for two one-character keys.

```apl
⍳[1]('a' 'b':1 2)           ⍝ ,⊂'ab'
⍳[1]((,¨'ab'):1 2)          ⍝ ,¨'ab'
```

`K:[axis]M` attaches keys to an existing axis. Store the result with `←`.

```apl
M←2 3⍴⍳6
M←'Jan' 'Feb' 'Mar':[2]M
⍳[2]M                       ⍝ 'Jan' 'Feb' 'Mar'
M[;'Feb']                   ⍝ 2 5
```

Several key vectors label successive axes. `[axes]` chooses their axes in the supplied order.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
⍴M                          ⍝ 2x 2x
⍳[1]M                       ⍝ 'alice' 'bob'
⍳[2]M                       ⍝ 'price' 'qty'
⍳[1 2]M                     ⍝ ('alice' 'bob' ⋄ 'price' 'qty')
M≡('price' 'qty' ⋄ 'alice' 'bob'):[2 1]2 2⍴10 2 20 4 ⍝ 1x
```

Monadic `:` removes all keys; `:[axes]` removes only the selected axes' keys.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
:M                          ⍝ 2 2⍴10 2 20 4
:[2]M                       ⍝ 'alice' 'bob':2 2⍴10 2 20 4
⍳[1 2](:M)                  ⍝ (1x 2x ⋄ 1x 2x)
```

Inside dfns, group functional colon: `(X:Y)` or `(:Y)`. A direct colon separates a guard.

```apl
{+/(:⍵)}'aa' 'bb':1 2       ⍝ 3
{('total':+/⍵)}1 2 3        ⍝ 'total':6
```

Keys must be unique strings on each axis: DOMAIN. A key-list length must match its axis: LENGTH.

| Expression | Error |
|---|---|
| `'aa' 'aa':1 2` | DOMAIN |
| `'aa' 'bb':[1]1 2 3` | LENGTH |

## Selection and assignment

A scalar selector removes its axis. A vector selector retains it. A character vector selects one string key.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
M['bob']                    ⍝ 'price' 'qty':20 4
M[2]                        ⍝ 'price' 'qty':20 4
M[;'qty']                   ⍝ 'alice' 'bob':2 4
M['alice';'qty']             ⍝ 2
M['bob';1]                  ⍝ 20
M[,⊂'alice']                ⍝ ('alice':[1]('price' 'qty':[2]1 2⍴10 2))
('bob' 'qty')⊃M             ⍝ 4
('bob' 'price')⌷M           ⍝ 20
M⍎'bob'                    ⍝ 'price' 'qty':20 4
```

Dot names are literal keys. `T.a.b` is `'b'⊃'a'⊃T`.

```apl
T←('n':1),('addr':'city':'Paris')
T.addr.city                 ⍝ 'Paris'
T.n+←1 ⋄ T.n                ⍝ 2
```

Plain named assignment appends missing rows or columns. New cells receive the array's prototype before assignment.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
M['cara']←30 5
M['cara';'qty']              ⍝ 5
M[;'tax']←1 2 3
M['alice';'tax']             ⍝ 1
```

Appending on several axes fills the new cross-cells too.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
M['cara';'tax']←3
M['cara']                   ⍝ 'price' 'qty' 'tax':0 0 3
M[;'tax']                   ⍝ 'alice' 'bob' 'cara':0 0 3
```

Missing keys on read or modified assignment: INDEX. Repeated selections on a keyed axis: DOMAIN. Numeric out-of-range assignment: INDEX.

For `T←'aa' 'bb':1 2`:

| Expression | Error |
|---|---|
| `T['cc']` | INDEX |
| `T['cc']+←1` | INDEX |
| `T[1 1]` | DOMAIN |
| `T[3]←9` | INDEX |

Scalar selection stores the whole RHS. A vector selection aligns its retained axis.

```apl
T←'aa' 'bb':1 2 ⋄ U←'bb' 'aa':8 9
T['aa']←U ⋄ T['aa']         ⍝ 'bb' 'aa':8 9
T[,⊂'aa']←U ⋄ T['aa']      ⍝ 9
```

## Agreement

Corresponding keyed axes align by union: left keys, then right-only keys. An absent counterpart receives the present value's prototype.

```apl
('aa':2)+('bb':3)            ⍝ 'aa' 'bb':2 3
('aa':2)×('bb':3)            ⍝ 'aa' 'bb':0 0
('aa':0)=('bb':0)            ⍝ 'aa' 'bb':1x 1x
```

Each keyed axis aligns independently, including reordered keys.

```apl
A←('alice':[1]'price':[2]1 1⍴10)
B←('bob':[1]'qty':[2]1 1⍴2)
A+B                         ⍝ ('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 0 0 2
('aa' 'bb':1 2)+('bb' 'aa':10 20) ⍝ 'aa' 'bb':21 12
```

With one axis unkeyed, agreement is positional. Expanding a keyed singleton drops that axis's labels. Enclosure broadcasts within the named value.

```apl
('base':5)+10 20 30          ⍝ 15 25 35
('base':5)+⊂10 20 30         ⍝ 'base':15 25 35
('aa' 'bb':1 2)+10 20        ⍝ 'aa' 'bb':11 22
```

Explicit axes choose the correspondence.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
M+[2]('qty' 'tax':10 2)      ⍝ ('alice' 'bob' ⋄ 'price' 'qty' 'tax'):2 3⍴10 12 2 20 14 2
```

## Array operations

Transpose moves axes with their keys.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
⍉M                          ⍝ ('price' 'qty' ⋄ 'alice' 'bob'):2 2⍴10 20 2 4
```

Reverse, rotate, take, drop and filtering move keys with their positions.

```apl
V←'aa' 'bb' 'cc':10 20 5
⌽V                          ⍝ 'cc' 'bb' 'aa':5 20 10
1⌽V                         ⍝ 'bb' 'cc' 'aa':20 5 10
¯1↑V                        ⍝ 'cc':5
1↓V                         ⍝ 'bb' 'cc':20 5
1 0 1/V                     ⍝ 'aa' 'cc':10 5
```

Reduction removes the reduced axis. Scan retains it. Each retains its frame keys.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
+/M                         ⍝ 'alice' 'bob':12 24
+⌿M                         ⍝ 'price' 'qty':30 6
+\'aa' 'bb' 'cc':1 2 3       ⍝ 'aa' 'bb' 'cc':1 3 6
+/¨'price' 'qty':(1 2 3 ⋄ 4 5 6) ⍝ 'price' 'qty':6 15
```

Shape-changing reshape drops keys. Same-shape reshape retains them. Ravel and table drop keys on merged/split axes, retaining unchanged axes.

```apl
V←'aa' 'bb':1 2
2⍴V                         ⍝ 'aa' 'bb':1 2
4⍴V                         ⍝ 1 2 1 2
M←'aa' 'bb':2 2⍴⍳4
,M                          ⍝ 1 2 3 4
⍳[1]⍪M                     ⍝ 'aa' 'bb'
```

Rank preserves frame keys. Assembled cell axes keep keys shared by every result cell in the same order.

```apl
V←'aa' 'bb':1 2
{⍵ ⍵}⍤0⊢V                  ⍝ 'aa' 'bb':2 2⍴1 1 2 2
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
⌽⍤1⊢M                      ⍝ ('alice' 'bob' ⋄ 'qty' 'price'):2 2⍴2 10 4 20
{10=↑⍵:⌽⍵ ⋄ ⍵}⍤1⊢M         ⍝ 'alice' 'bob':2 2⍴2 10 20 4
```

Per-cell rotation drops keys on the rotated axis when cells use different rotations.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
0 1⌽M                       ⍝ 'alice' 'bob':2 2⍴10 2 4 20
```

Outer product retains both arguments' axes. Array-valued power counts supply the result frame.

```apl
('aa' 'bb':1 2)+⌝('xx' 'yy':10 20) ⍝ ('aa' 'bb' ⋄ 'xx' 'yy'):2 2⍴11 21 12 22
N←'initial' 'once' 'twice':0 1 2
(2∘×)⍣N⊢3                  ⍝ 'initial' 'once' 'twice':3 6 12
```

Inner product, Decode and matrix solve align contracted keys. Both keyed: equal key sets required (LENGTH otherwise). Either unkeyed: positional contraction.

```apl
('hi' 'lo':1 2)+.×('lo' 'hi':10 20) ⍝ 40
('hi' 'lo':10 10)⊥('lo' 'hi':2 1) ⍝ 12
M←('r1' 'r2' ⋄ 'xx' 'yy'):2 2⍴2x 0x 0x 4x
('r2' 'r1':12x 4x)⌹M        ⍝ 'xx' 'yy':2x 3x
⌹M                          ⍝ ('xx' 'yy' ⋄ 'r1' 'r2'):2 2⍴1r2 0x 0x 1r4
```

Catenate joins keys on its joined axis and aligns other keyed axes. An unkeyed joined axis makes the result axis unkeyed.

```apl
('aa':1),('bb':2)            ⍝ 'aa' 'bb':1 2
('aa' 'bb':1 2),5            ⍝ 1 2 5
A←('aa' 'bb' ⋄ 'xx' 'yy'):2 2⍴1 2 3 4
B←('bb' 'aa' ⋄ ,⊂'zz'):2 1⍴5 6
A,B                         ⍝ ('aa' 'bb' ⋄ 'xx' 'yy' 'zz'):2 3⍴1 2 6 3 4 5
```

Replicate can repeat an unkeyed axis while retaining keys on other axes.

```apl
2/'aa' 'bb':2 2⍴1 2 3 4     ⍝ 'aa' 'bb':2 4⍴1 1 2 2 3 3 4 4
```

Repeated or invented positions on a keyed axis give DOMAIN. For `V←'aa' 'bb':1 2`:

| Expression | Reason |
|---|---|
| `V,V` | Duplicate joined keys |
| `2/V` | Repeated keys |
| `3↑V` | Unnamed padding |
| `1 0 1\V` | Unnamed inserted position |

### Windows

Full windows have an unkeyed position axis. Each window retains its slice's keys; assembly keeps only keys shared by all windows. Stencil frames retain their centre keys. Padding drops keys on the padded window axis.

```apl
V←'aa' 'bb' 'cc':1 2 3
2↕V                         ⍝ 2 2⍴1 2 2 3
3↕V                         ⍝ 'aa' 'bb' 'cc':[2]1 3⍴1 2 3
{+/⍵}⌺3⊢V                  ⍝ 'aa' 'bb' 'cc':3 6 5
{(⍳[1]⍵)≡'aa' 'bb' 'cc'}⌺3⊢V ⍝ 'aa' 'bb' 'cc':0x 1x 0x
```

## Match and search

Match aligns keys independently on every keyed axis. Shapes and unkeyed axes must match positionally. Searches compare values or cells. Found positions return axis keys.

```apl
V←'low' 'mid' 'high':10 20 30
V≡⌽V                        ⍝ 1x
V⍳20 99                     ⍝ ('mid' ⋄ 4x)
V⍸5 15 25 40                ⍝ (0x ⋄ 'low' ⋄ 'mid' ⋄ 'high')
⍒V                          ⍝ 'high' 'mid' 'low'
V[⍒V]                       ⍝ 'high' 'mid' 'low':30 20 10
('aa':7)∊('bb':7)            ⍝ 'aa':1x
```

Multidimensional positions mix keys and numeric coordinates. Bare Iota generates from values as usual.

```apl
M←'price' 'qty':[2]2 2⍴10 2 20 4
↑⍸M=20                      ⍝ (2x ⋄ 'price')
⍳('rows' 'cols':2 3)        ⍝ ⍳2 3
```

Unique and set operations compare values and retain the selected positions' keys.

```apl
A←'aa' 'bb':1 2 ⋄ B←'xx' 'yy':2 3
A∩B                         ⍝ 'bb':2
A~B                         ⍝ 'aa':1
A∪B                         ⍝ 'aa' 'bb' 'yy':1 2 3
∪'aa' 'bb' 'cc':1 1 2       ⍝ 'aa' 'cc':1 2
```

Monadic Key passes named positions to its operand. Dyadic Key preserves labels on the grouped values.

```apl
V←'aa' 'bb' 'cc':1 2 1
{⊂⍵}⌸V                     ⍝ (('aa' 'cc') ⋄ ,⊂'bb')
1 2 1{⊂⍵}⌸V                ⍝ (('aa' 'cc':1 1) ⋄ ('bb':[1],2))
```

## Named axes

An axis name identifies a dimension: `city`. Its keys identify positions: `Paris London`. Names are unique within an array.

```apl
M←'city' 'month':[0]2 3⍴⍳6
⍳[0]M                         ⍝ 'city' 'month'
+/['month']M                   ⍝ 'city':[0]6 15
```

Axis `0` refers to the list of axes. `N:[0]M` supplies one name per axis; use an axis's own index to leave it unnamed. `:[0]M` removes names. These return new values sharing the element buffer. Assign with `M←…` to store the result.

```apl
M←'city' 'month':[0]2 3⍴⍳6
⍳[0]('city' ⋄ 2):[0]M         ⍝ ('city' ⋄ 2x)
⍳[0](:[0]M)                  ⍝ 1x 2x
(⍳[0]M):[0](:[0]M)           ⍝ 'city' 'month':[0]2 3⍴⍳6
```

A keyed vector on the left of `:` supplies both axis names and position keys. Its keys name axes; its values list their positions.

```apl
sales←('city' 'month':('London' 'Paris' ⋄ 'Jan' 'Feb' 'Mar')):2 3⍴10 20 30 40 50 60
⍳[0]sales                     ⍝ 'city' 'month'
⍳[1 2]sales                   ⍝ ('London' 'Paris' ⋄ 'Jan' 'Feb' 'Mar')
sales['Paris';'Feb']           ⍝ 50
```

Reuse descriptions, or attach them to selected axes. Other axes keep their names and keys.

```apl
months←'month':'Jan' 'Feb' 'Mar'
M←months:[2]2 3⍴⍳6
⍳[0]M                        ⍝ (1x ⋄ 'month')
⍳[2]M                        ⍝ 'Jan' 'Feb' 'Mar'
⍳[0]('city':'London' 'Paris'):M ⍝ 'city' 'month'
```

```apl
axes←'month' 'city':('Jan' 'Feb' 'Mar' ⋄ 'London' 'Paris')
M←axes:[2 1]2 3⍴⍳6
⍳[0]M                        ⍝ 'city' 'month'
⍳[1 2]M                      ⍝ ('London' 'Paris' ⋄ 'Jan' 'Feb' 'Mar')
```

Agreement first pairs equal names, wherever they occur. Unpaired axes broadcast. Result order is left axes, then right-only axes.

```apl
M←'city' 'month':[0]2 3⍴⍳6
N←'month':[0]10 20 30
M+N                           ⍝ 'city' 'month':[0]2 3⍴11 22 33 14 25 36
⍳[0]N+M                      ⍝ 'month' 'city'
M+⍉M                         ⍝ 'city' 'month':[0]2 3⍴2 4 6 8 10 12
```

Remaining axes pair in leading-axis order. A named/unnamed pair keeps the name. Two different names remain separate broadcast dimensions. Length agreement and position-key alignment then apply.

```apl
M←'city' 'month':[0]2 3⍴⍳6
M+10 20                       ⍝ 'city' 'month':[0]2 3⍴11 12 13 24 25 26
A←('city' ⋄ 2):[0]2 3⍴1
B←('product' ⋄ 2):[0]4 3⍴2
⍳[0]A+B                      ⍝ ('city' ⋄ 2x ⋄ 'product')
⍴A+B                          ⍝ 2x 3x 4x
```

Function qualifiers select axes by name. Array brackets select positions on those axes.

```apl
M←'city' 'month':[0]('Paris' 'London' ⋄ 'Jan' 'Feb' 'Mar'):2 3⍴⍳6
⍳[1]'Paris'⌷['city']M        ⍝ 'Jan' 'Feb' 'Mar'
M['Paris']≡'Paris'⌷['city']M  ⍝ 1x
⍳[0]⊂['month']M              ⍝ ,⊂'city'
+/['city' 'month']M            ⍝ ⊂21
```

Transpose moves names. Scalar selection removes the selected axis. Repeating positions retains names. Shape-changing reshape and axis merging drop names on replaced axes; new axes start unnamed.

```apl
M←'city' 'month':[0]2 3⍴⍳6
⍳[0]⍉M                      ⍝ 'month' 'city'
⍳[0]M[1]                     ⍝ ,⊂'month'
⍳[0]2 3⍴M                   ⍝ 'city' 'month'
⍳[0]3 2⍴M                   ⍝ 1x 2x
⍳[0],M                       ⍝ ,1x
V←'city':[0]1 2
⍳[0]2/V                      ⍝ ,⊂'city'
⍳[0]⍪V                      ⍝ ('city' ⋄ 2x)
```

Duplicating an axis name drops every occurrence of it. Position keys remain.

```apl
V←'city':[0]'Paris' 'London':1 2
W←V×⌝V
⍳[0]W                        ⍝ 1x 2x
⍳[1 2]W                      ⍝ ('Paris' 'London' ⋄ 'Paris' 'London')
```

| Expression | Error |
|---|---|
| `+/['missing']M` | INDEX |
| `'city' 'city':[0]2 3⍴0` | DOMAIN |
| `'city':[0]2 3⍴0` | LENGTH |

## Python and JSON

Python dicts become keyed vectors recursively. `.py` converts them back to dicts.

```python
from basedpl import Array
import numpy as np

record = Array({'price': 10, 'address': {'city': 'Paris'}})
assert record.py == {'price': 10, 'address': {'city': 'Paris'}}
assert record.axis_keys == (('price', 'address'),)
```

Attach position keys with `axis_keys`, axis names with `axis_names`. `None` leaves an axis unkeyed/unnamed. The corresponding properties inspect them; `.np` copies the values to NumPy.

```python
rows, cols = ['London', 'Paris'], ['Jan', 'Feb', 'Mar']
sales = Array([[10, 20, 30], [40, 50, 60]], axis_keys=(rows, cols), axis_names=('city', 'month'))
assert sales.axis_keys == (tuple(rows), tuple(cols))
assert sales.axis_names == ('city', 'month')
assert sales.np.tolist() == [[10, 20, 30], [40, 50, 60]]
columns_only = Array(sales.np, axis_keys=(None, cols), axis_names=(None, 'month'))
assert columns_only.axis_keys == (None, tuple(cols))
assert columns_only.axis_names == (None, 'month')
```

`.df` converts to pandas. For a matrix, keys label rows/columns and axis names name the index/columns. `.py` uses this conversion for keyed arrays of rank ≥2. Install pandas with `pip install 'basedpl[pandas]'`; it is imported on conversion.

```python
df = sales.df
assert df.loc['Paris', 'Feb'] == 50
assert (df.index.name, df.columns.name) == ('city', 'month')
assert sales.py.equals(df)
assert columns_only.df.index.tolist() == [1, 2]
```

Above rank 2, the last axis supplies columns; earlier axes form a row MultiIndex. Unkeyed axes use 1-origin labels. Scalars and vectors become one-column DataFrames.

```python
cube = Array(np.arange(12).reshape(2, 2, 3), axis_keys=(None, rows, cols), axis_names=('year', 'city', 'month'))
assert cube.df.index.names == ['year', 'city']
assert cube.df.loc[(2, 'Paris'), 'Mar'] == 11
assert Array({'aa': 1, 'bb': 2}).df.loc['bb', 1] == 2
assert Array(7).df.loc[1, 1] == 7
```

[`•JSON`](data.md#json) converts ordinary JSON objects to keyed vectors. The [process protocol](processes.md) carries `axis_keys` and `axis_names` beside shape, row-major data and prototype. For `sales` above:

```json
{
  "shape": [2, 3],
  "data": [10, 20, 30, 40, 50, 60],
  "prototype": 0,
  "axis_keys": [["London", "Paris"], ["Jan", "Feb", "Mar"]],
  "axis_names": ["city", "month"]
}
```

Use `null` for an unkeyed/unnamed axis. Omit each metadata field when none of its axes has that metadata:

```json
{"shape":[1,2],"data":[10,20],"prototype":0,"axis_keys":[null,["Jan","Feb"]]}
{"shape":[2],"data":[10,20],"prototype":0}
```
