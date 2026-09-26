

# `⌷` — Index

`I⌷Y` indexes `Y`. The left argument has one item for each leading axis
of `Y`. Axes that it omits are taken whole. `⌷Y` is identity. An array
next to an argument selects along its leading axis: `Y I` is `[I;]⌷Y`.
After an array, [Dot](dot.qmd) indexing is `⌷`: `x.(I)` is `(I)⌷x`.

``` apl
m←3 4⍴⍳12
1⌷m                ⍝ 4 5 6 7
1 2⌷m              ⍝ 6
m 1                ⍝ 4 5 6 7
```

Each item of the left argument is a single position, an array of
positions, or `∞`. A single position removes its axis from the result.
An array of positions replaces the axis with its own shape. An array of
positions for one axis goes in a one-item list, as in `[2 0;]`, because
a plain vector gives one position for each axis.

``` apl
v←10 20 30
[2 0;]⌷v           ⍝ 30 10
m←3 4⍴⍳12
[1 2;0 3]⌷m        ⍝ [4 7 ⋄ 8 11]
```

Positions count from 0. Negative positions count from the end, so `¯1`
is the last position.

``` apl
v←10 20 30
¯1⌷v               ⍝ 30
[¯1 0;]⌷v          ⍝ 30 10
```

An array of positions whose elements are vectors does choose indexing.
Each vector is one coordinate, with one position for each axis of `Y`.

``` apl
m←3 4⍴⍳12
[[0 1;2 ¯1];]⌷m    ⍝ 1 11
m [0 1;2 ¯1]       ⍝ 1 11
```

`∞` takes a whole axis, and `¯∞` takes it in reverse order. They are
valid only as a whole item, not inside an array of positions.

``` apl
m←3 4⍴⍳12
∞ 1⌷m              ⍝ 1 5 9
¯∞ ∞⌷m             ⍝ ⊖3 4⍴⍳12
```

On a keyed axis, a string is one key.

``` apl
T←"aa" "bb":1 2
"bb"⌷T             ⍝ 2
T "bb"             ⍝ 2
```

Assignment through `⌷` replaces the selected items. A missing key is
added, even on an axis with no keys.

``` apl
x←10 20 30
(¯1⌷x)←9
x                  ⍝ 10 20 9
T←"aa" "bb":1 2
("cc"⌷T)←3
T                  ⍝ "aa" "bb" "cc":1 2 3
```

With an axis, `⌷⍠K` applies the left argument’s items to the axes `K`.

``` apl
1⌷⍠1 (3 4⍴⍳12)      ⍝ 1 5 9
```

## Errors

- `RANK`: left argument not a scalar or vector
- `LENGTH`: more items than axes
- `INDEX`: position outside its axis, missing key, a scalar applied to
  an argument
- `DOMAIN`: non-integral position, `∞` or `¯∞` inside an array of
  positions
