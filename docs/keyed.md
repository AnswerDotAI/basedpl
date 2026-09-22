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

Several key vectors label successive axes. `[axes]` chooses which axes to label and always attaches to existing dimensions.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
⍴M                          ⍝ 2x 2x
⍳[1]M                       ⍝ 'alice' 'bob'
⍳[2]M                       ⍝ 'price' 'qty'
:M                          ⍝ 2 2⍴10 2 20 4
⍳[2](:[2]M)                 ⍝ 1x 2x
```

Monadic `:` removes axis keys. Inside dfns, group functional colon: `(X:Y)` or `(:Y)`. A direct colon separates a guard.

Keys must be unique strings on each axis: DOMAIN. A key-list length must match its axis: LENGTH.

## Selection and assignment

A scalar selector removes its axis. A vector selector retains it. A character vector selects one string key.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
M['bob']                    ⍝ 'price' 'qty':20 4
M[;'qty']                   ⍝ 'alice' 'bob':2 4
M['alice';'qty']             ⍝ 2
M[,⊂'alice']                ⍝ ('alice':[1]('price' 'qty':[2]1 2⍴10 2))
('bob' 'qty')⊃M             ⍝ 4
('bob' 'price')⌷M           ⍝ 20
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

Missing keys on read or modified assignment: INDEX. Repeated selections on a keyed axis: DOMAIN. Numeric out-of-range assignment: INDEX.

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

With one axis unkeyed, agreement is positional. Expanding a keyed singleton drops that axis's labels. Enclosure broadcasts within the named value.

```apl
('base':5)+10 20 30          ⍝ 15 25 35
('base':5)+⊂10 20 30         ⍝ 'base':15 25 35
```

Explicit axes choose the correspondence.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
M+[2]('qty' 'tax':10 2)      ⍝ ('alice' 'bob' ⋄ 'price' 'qty' 'tax'):2 3⍴10 12 2 20 14 2
```

## Array operations

Transpose moves axes with their labels. Reverse, rotate and filtering move labels with their positions. Reduction removes the reduced axis. Scan retains it. Shape-changing reshape drops keys.

```apl
M←('alice' 'bob' ⋄ 'price' 'qty'):2 2⍴10 2 20 4
+/M                         ⍝ 'alice' 'bob':12 24
+⌿M                         ⍝ 'price' 'qty':30 6
4⍴'aa' 'bb':1 2             ⍝ 1 2 1 2
```

Rank preserves frame labels. Assembled cell axes keep labels shared by every result cell in the same order. Outer product retains both arguments' axes.

Per-cell rotation drops labels on the rotated axis when its cells use different rotations. Other axes retain their labels.

Inner product, Decode and matrix solve align keyed contracted axes by name. Key sets must match: LENGTH. With either axis unkeyed, contraction is positional.

```apl
('hi' 'lo':1 2)+.×('lo' 'hi':10 20) ⍝ 40
```

Catenate joins positions. Duplicate joined keys give DOMAIN. Replicate may filter a keyed axis, but repetition or padding on that axis gives DOMAIN.

## Match and search

Match aligns keys independently on every keyed axis. Shapes and unkeyed axes must match positionally. Searches compare values or cells. Found positions return axis keys.

```apl
V←'low' 'mid' 'high':10 20 30
V≡⌽V                        ⍝ 1x
V⍳20 99                     ⍝ ('mid' ⋄ 4x)
V⍸5 15 25 40                ⍝ (0x ⋄ 'low' ⋄ 'mid' ⋄ 'high')
⍒V                          ⍝ 'high' 'mid' 'low'
('aa':7)∊('bb':7)            ⍝ 'aa':1x
```

Monadic Key passes named positions to its operand. Dyadic Key preserves labels on the grouped values.

```apl
V←'aa' 'bb' 'cc':1 2 1
{⊂⍵}⌸V                     ⍝ (('aa' 'cc') ⋄ ,⊂'bb')
```

## Python and JSON

Python dicts become keyed vectors recursively. `Array(data, axis_keys=[rows, cols])` attaches labels to existing axes. Use `None` for an unkeyed axis.

`.axis_keys` returns a tuple of label tuples or `None`. `.np` copies values to an ndarray. `.py` returns a dict for a keyed vector and a pandas DataFrame for keyed arrays of rank ≥2.

`.df` converts any rank to a DataFrame. The last axis supplies columns. Earlier axes supply rows, using a MultiIndex above rank 2. Unkeyed axes use 1-origin labels. Pandas is an optional, lazy dependency: `pip install 'basedpl[pandas]'`.

The [process protocol](processes.md) adds `axis_keys`, e.g. `[null, ["price", "qty"]]`. All-unkeyed arrays omit it.
