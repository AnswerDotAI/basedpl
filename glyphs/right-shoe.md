

# `⊃` — Mix / Pick

Function. Keys: `Alt-x`. Hanzi: `选/合` (`選/合`). Ranks: `0` monadic,
`1 ∞` dyadic

`⊃Y` assembles items into rectangular cells, padding to a common shape
with fill.

``` bpl
⊃1 2 3             ⍝ 1 2 3
⊃[[1 2] [3 4 5]]   ⍝ [1 2 0⋄3 4 5]
```

The result keeps the keys and axis names that the cells agree on. A cell
without keys or a name agrees with any, as in arithmetic. An axis that
needs padding loses its keys.

``` bpl
⊃[10 20;["a":1 "b":2]]   ⍝ ["a":10 "b":20⋄"a":1 "b":2]
```

`P⊃Y` follows the path `P` into `Y`, one level for each item of `P`. A
level is a position or key, or a vector of them with one for each
leading axis it selects. With fewer positions than axes, a level selects
the cell made of the remaining axes. With one for every axis, it selects
that item. The next level enters a nested item. An empty path returns
`Y`. `1 1⊃m` takes row 1 of `m`, then item 1 of that row.

``` bpl
1⊃2 3⍴⍳6ₓ           ⍝ [3 4 5]ₓ
[[1 1]]⊃2 3⍴⍳6ₓ     ⍝ 4ₓ
1 1⊃2 3⍴⍳6ₓ         ⍝ 4ₓ
1 0⊃[[1 2] [3 4]]  ⍝ 3
⍬⊃1 2              ⍝ 1 2
```

A level can use strings on [keyed axes](../keyed.ipynb). A path enters
nested records as dot access does: `["a" "b"]⊃T` is `T.a.b`.

``` bpl
T←["price":1 2 3;"qty":4 5 6]
"qty"⊃T             ⍝ 4 5 6
"qty" 1⊃T           ⍝ 5
```

A level selects one item or cell. To read several items, use
[Dot](dot.qmd) indexing: `x.[[0 2] [1 3]]` pairs two arrays of
positions.

With axes, `⊃⍠A Y` places the cell axes of Mix among the frame axes. See
[Axis](quad-colon.qmd). On Pick, negative positions count from the end.
Position 0 or ¯1 on an empty axis gives fill, as does
[First](up-arrow.qmd).

## Inverse

`⊃⁻¹Y` splits `Y` into vectors along its last axis.

``` bpl
⊃⁻¹[1 2⋄3 4]   ⍝ [1 2;3 4]
```

## Errors

- `RANK`: Mix axes not a unit or vector, a path not a unit or vector, a
  level with more positions than its array has axes
- `LENGTH`: Mix axis count not the cell rank
- `INDEX`: position outside the array, missing key, or Mix axis outside
  the result
- `DOMAIN`: repeated or fractional Mix axes, a level that holds an array
  of positions
