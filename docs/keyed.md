# Keyed arrays

[Glyph index](index.md#glyph-reference)

A keyed array is an array of any rank in which every element has a unique string key as well as a coordinate. Its elements are the values. Numeric selectors use coordinates. String selectors use keys. Keyed arrays hold named data. They do not hold scope. bAsedPL has no namespaces.

## Construction

`(key:value ⋄ key:value)` builds a keyed vector. Both sides of `:` are expressions. Entries evaluate left to right, key before value. A repeated key keeps its first position and takes its last value. `()` is the empty keyed vector. Inside a dfn, `:` is a guard.

```apl
T←('price':1 2 3 ⋄ 'qty':4 5 6)
≢T                          ⍝ 2x
⍳T                          ⍝ 'price' 'qty'
⍳('a':1 ⋄ 'b':2 ⋄ 'a':3)    ⍝ (,'a') (,'b')
≢()                         ⍝ 0x
```

One string names the whole right value in a one-entry vector. An array of strings names value cells. Its shape must be a prefix of the value shape. A strand of characters is one string. One-character keys in an array need `,¨`.

```apl
⍴('a':11 12 13)             ⍝ ,1x
('price' 'qty':2 3⍴⍳6)≡('price':1 2 3 ⋄ 'qty':4 5 6) ⍝ 1x
((,¨'ab'):1 2)≡('a':1 ⋄ 'b':2) ⍝ 1x
⍳('a' 'b':1 2)              ⍝ ,⊂'ab'
M←((2 2⍴'nw' 'ne' 'sw' 'se'):2 2⍴⍳4)
⍴M                          ⍝ 2x 2x
'sw'⊃M                      ⍝ 3
```

Keys that are not strings give DOMAIN. Duplicate keys above rank 1 give DOMAIN. A value of lower rank than the key array gives RANK. A key shape that is not a prefix of the value shape gives LENGTH. Several `⋄` entries must each be vectors.

## Selection

Brackets return a keyed array with the selector's shape. One selected element is a keyed scalar. Pick, dot and dyadic `⍎` return values.

```apl
T←('price':1 2 3 ⋄ 'qty':4 5 6)
T['qty' 'price']            ⍝ ('qty':4 5 6 ⋄ 'price':1 2 3)
⍴T['qty']                   ⍝ 0⍴0x
'qty'⊃T                     ⍝ 4 5 6
2⊃T                         ⍝ 4 5 6
('qty' 'qty')⊃T             ⍝ (4 5 6 ⋄ 4 5 6)
T.qty                       ⍝ 4 5 6
T⍎'qty'                     ⍝ 4 5 6
(⍳T)⊃T                      ⍝ (1 2 3 ⋄ 4 5 6)
```

`⍳T` is the ordinary array of keys, with the shape of `T`. `(⍳T)⊃T` is the ordinary array of values.

`T.a.b` is `'b'⊃'a'⊃T`. Names after the dot are literal keys. The dot applies after a name that holds an array, a parenthesised value or a keyed literal, each optionally followed by index brackets. Between functions `.` is inner product.

A missing key gives INDEX. Brackets that select one element twice give DOMAIN. A selector that mixes coordinates and keys gives DOMAIN.

## Assignment

```apl
T←('n':1 ⋄ 'addr':('city':'Paris'))
T.n+←1
T.addr.city←'Rome'
T.addr.zip←'75'
T['n']←('n':10 ⋄ 'x':0)
T['tax']←2
T                           ⍝ ('n':10 ⋄ 'addr':('city':'Rome' ⋄ 'zip':'75') ⋄ 'tax':2)
```

Bracket assignment replaces the selected elements. A keyed right side supplies each selected key's own value. Its order and extra keys are ignored. A key it lacks gives INDEX. An unkeyed right side assigns by position. Selecting one element twice gives DOMAIN.

Pick and dot assignment replace a whole value.

With plain `←`, all three forms append a missing key to a keyed vector. Bracket assignment appends missing keys in selector order. Appending to any other rank gives RANK. A coordinate names no key, so `T[9]←v` gives INDEX. Modified assignment needs an existing key. Every container on the path must already exist.

Arrays have value semantics. Updating one name leaves other copies unchanged.

## Array operations

Scalar functions, Each, scan, reverse, rotate, transpose, take, drop, replicate, reshape, ravel and bracket indexing keep keys. Reduction, inner product, mix, split, enlist and grade return ordinary arrays. A result that would repeat a key or need a new one gives DOMAIN. Examples are `T[1 1]`, `4⍴T`, `2/T`, `3↑T` and `T,5`.

```apl
T←('price':1 2 3 ⋄ 'qty':4 5 6)
10×T                        ⍝ ('price':10 20 30 ⋄ 'qty':40 50 60)
+/¨T                        ⍝ ('price':6 ⋄ 'qty':15)
+/T                         ⍝ 5 7 9
⌽T                          ⍝ ('qty':4 5 6 ⋄ 'price':1 2 3)
1 0/T                       ⍝ ('price':1 2 3)
T,('qty':0 ⋄ 'tax':2)       ⍝ ('price':1 2 3 ⋄ 'qty':0 ⋄ 'tax':2)
```

Catenating two keyed vectors merges by key. The right value wins and the key keeps its first position. Catenation at higher ranks keeps the ordinary result shape and needs distinct keys.

Rank keeps keys only for rank-0 cells with scalar results. Outer product keeps keys only when the other argument is an unkeyed scalar.

## Agreement

Two keyed arguments of rank 0 or 1 align by key. The result has the left keys, then the right-only keys. A missing counterpart is the fill of the value that is present: zero, space or a nested fill. Two keyed arguments of higher rank need equal shapes and equal key sets. Their result uses the left layout.

```apl
('a':1 ⋄ 'b':2)+('b':20 ⋄ 'a':10) ⍝ ('a':11 ⋄ 'b':22)
('a':2)×('b':3)             ⍝ ('a':0 ⋄ 'b':0)
('a':0)=('b':0)             ⍝ ('a':1x ⋄ 'b':1x)
```

One keyed argument uses [ordinary agreement](rules.md#agreement-and-pervasion). The result keeps the keys exactly when it has the keyed argument's shape. Each and Rank follow the same rules.

```apl
T←('price':1 2 3 ⋄ 'qty':4 5 6)
2+T                         ⍝ ('price':3 4 5 ⋄ 'qty':6 7 8)
T+⊂10 20 30                 ⍝ ('price':11 22 33 ⋄ 'qty':14 25 36)
('a':1 ⋄ 'b':2)+2 3⍴0       ⍝ 2 3⍴1 1 1 2 2 2
```

## Match, search and sets

Match needs the same shape, the same key set and matching values. It ignores the arrangement of keys within the shape. A keyed array never matches an unkeyed array.

Between two keyed arrays, an element is a member when its key is present with a matching value. With one keyed argument, searches compare values and ignore keys. Membership keeps the keys of its left argument. Intersection and Without keep the surviving left entries. Union of two keyed vectors needs shared keys to hold matching values, otherwise DOMAIN. Union with one keyed argument returns an ordinary vector. Index-of returns ordinary positions.

```apl
A←('a':1 ⋄ 'b':2)
B←('a':1 ⋄ 'b':9 ⋄ 'c':3)
A≡('b':2 ⋄ 'a':1)           ⍝ 1x
A≡1 2                       ⍝ 0x
A∊B                         ⍝ ('a':1x ⋄ 'b':0x)
A∩B                         ⍝ ('a':1)
A~B                         ⍝ ('b':2)
A∪('c':3)                   ⍝ ('a':1 ⋄ 'b':2 ⋄ 'c':3)
A∊1                         ⍝ ('a':1x ⋄ 'b':0x)
A⍳2                         ⍝ 2x
```

## Display, Python and process interfaces

Display and `⍕` show each key with its value. A keyed vector displays as its constructor.

Python dicts with string keys convert to keyed vectors, recursively and in order. `Array.py` returns a dict in ravel order for a keyed array of any rank. `Array.shape` and `Array.np` keep the outer shape. `Array.np` holds the values only.

```python
from basedpl import Session
with Session() as apl:
    t = apl("('b':1 ⋄ 'a':('x':'hi'))")
    assert t.py == {'b': 1, 'a': {'x': 'hi'}}
    assert apl('t.a.x', t={'a': {'x': 'hi'}}).py == 'hi'
```

The [process interfaces](processes.md) encode a keyed array like any other array and add a `keys` list in ravel order, one string per element. Shape and keys round-trip unchanged.
