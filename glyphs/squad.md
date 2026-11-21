

# `⌷` — Materialise / Index

Function. Keys: `Alt-q Backslash`. Hanzi: `索`. Ranks: `∞` monadic,
`1 ∞` dyadic

`I⌷Y` indexes `Y`. `I` lists selectors for leading axes of `Y`. Omitted
axes are taken whole. `⌷Y` is identity. A
[subscript](../scripts.ipynb#subscripts) writes a single position: `m₁`
is `1⌷m`. After an array, [Dot](dot.qmd) indexing `x.[I]` pairs arrays
of positions with each other instead: `m.[[0 2] [1 3]]` reads the items
at 0,1 and 2,3, whereas `[[0 2] [1 3]]⌷m` reads a 2 by 2 block.

``` bpl
m←3 4ρι12ₓ
1⌷m                ⍝ [4 5 6 7]ₓ
1 2⌷m              ⍝ 6ₓ
```

Each item of `I` is a single position, an array of positions, an array
of coordinates, `:`, `∞` or `¯∞`. Selecting with a single position drops
that axis from the result. Selecting with an array of positions puts the
array’s shape in place of the axis. Arrays of positions for several axes
give their cross product. An array of positions for one axis is one
item, as in `[[2 0]]`, because `[2 0]` gives a position on each of two
axes.

``` bpl
v←10 20 30
[[2 0]]⌷v          ⍝ 30 10
m←3 4ρι12ₓ
[1 2;0 3]⌷m        ⍝ [4 7◇8 11]ₓ
```

Positions count from 0. Negative positions count from the end. `¯1` is
the last position.

``` bpl
v←10 20 30
¯1⌷v               ⍝ 30
[[¯1 0]]⌷v         ⍝ 30 10
```

`:` takes one whole input axis. `∞` takes all remaining axes, as
Python’s `...` does. At most one `∞` is allowed. `¯∞` inserts an
unnamed, unkeyed axis of length one without consuming an input axis.
Write each marker as a separate bracket item: `[: 1]`, not `[:1]`, which
applies Unkey to `1`. [Reverse](circle-stile.qmd) `⌽` reverses the last
axis:

``` bpl
m←3 4ρι12ₓ
[: 1]⌷m            ⍝ [1 5 9]ₓ
ρ[¯∞ ∞]⌷m          ⍝ [1 3 4]ₓ
ρ[: ¯∞ :]⌷m        ⍝ [3 1 4]ₓ
ρm.[∞ ¯∞]          ⍝ [3 4 1]ₓ
⌽₀m                ⍝ [8 9 10 11◇4 5 6 7◇0 1 2 3]ₓ
```

On a keyed axis, a string is one key.

An array selector can repeat a position or key, and gives one item for
each of its items. When it repeats an item of a keyed axis, by key or by
position, the result’s axis has no keys: for `T←"a" "b":1 2`,
`T.[["a" "a"]]` gives `1 1`, as `T.[[0 0]]` does. The keys of an unnamed
selector are not kept: the result has the keys of `Y` at the positions
selected. A named selector gives the result its axis names and keys, as
described below.

``` bpl
T←"aa" "bb":1 2
"bb"⌷T             ⍝ 2
```

An array of coordinates, as `⍸` gives, selects the item or cell at each
coordinate. A coordinate holds a position or key for each of several
leading axes, and the next item of `I` selects along the axis after
them. A coordinate longer than the rank continues into the item it
reaches, as a path of [Pick](right-shoe.qmd) does.

``` bpl
m←3 4ρι12ₓ
[⍸m>9]⌷m           ⍝ [10 11]ₓ
a←2 3 4ρι24ₓ
[[[0 1] [1 2]] [3 0]]⌷a   ⍝ [7 4◇23 20]ₓ
[[[1 2] [0 0]]]⌷[[10 20] [30 40 50]]   ⍝ 50 10
```

A path array’s axes precede the cells it reaches. `¯∞` before the path
array inserts before those axes. After paths enter nested items, more
selectors select within each reached cell, as they would from that cell
alone, using its own keys and names. Their selections must agree in
shape:

``` bpl
nest←[[1 2 3◇4 5 6] [7 8 9◇10 11 12]]
paths←[[0 1] [1 0]]
ρ[¯∞ paths]⌷nest       ⍝ [1 2 3]ₓ
ρ[paths ¯∞]⌷nest       ⍝ [2 1 3]ₓ
ρ[paths : ¯∞]⌷nest     ⍝ [2 3 1]ₓ
ρnest.[paths ∞ ¯∞]     ⍝ [2 3 1]ₓ
[paths 0]⌷nest         ⍝ 4 7
```

An array of positions with named axes is a named selector, such as a
[named index](iota.qmd) from `ι["i":n]` or
[`•index`](../system/index.qmd). A uniquely named selector selects along
the axis of `Y` with that name, wherever it stands in `I`. Without such
an axis, it takes the next free axis that has no name. Its axes replace
the axis it selects, with their names and any keys it has. A named
vector without keys keeps the keys at the positions it selects.
Selectors that share a name pair with each other, as arithmetic pairs
named axes. Together they take the free axes in order, as unnamed
selectors do. `[i i]⌷m` is therefore the diagonal. With `k` along axis
`j`, `[k j]⌷m` reads row `k` of each column, as NumPy’s `m[k, j]` does.
Selectors with different names, and unnamed selectors, combine as above.
[Indexing](../indexing.ipynb) shows everyday uses.

``` bpl
m←3 3ρι9ₓ ◇ i←ι["i":3] ◇ j←ι["j":3] ◇ k←["j":3]ρ2 0 1
[i i]⌷m            ⍝ ["i":3]ρ[0 4 8]ₓ
ρ[i j]⌷m           ⍝ ["i":3 "j":3]ₓ
[k j]⌷m            ⍝ ["j":3]ρ[6 1 5]ₓ
sales←["city":"NY" "LA";"month":"Jan" "Feb" "Mar"]:[10 20 30◇40 50 60]
t←ι⍠"month" sales
sales≡[t]⌷sales     ⍝ $t
```

For a named selector too, negative positions count from the end. Index
reads fill at a position outside its axis or not whole, and a write
there writes nothing, as [Indexing](../indexing.ipynb#positions)
describes. A missing key also reads fill. [`⍠`](quad-colon.qmd#fill)
sets that fill. Other edge rules are arithmetic on positions, which
[`•edge`](../system/edge.qmd) provides. By default `•edge` gives NaN
outside the axis:

``` bpl
x←10ₓ×↦5ₓ ◇ i←ι["i":5] ◇ clamp←•edge "clamp"
x.[i-1]                        ⍝ ["i":5]ρ[50 10 20 30 40]ₓ
x.[5•edge⁝i-1]               ⍝ ["i":5]ρ[0 10 20 30 40]ₓ
x.[(5 clamp i-1)]              ⍝ ["i":5]ρ[10 10 20 30 40]ₓ
[5•edge⁝i-1]⌷⍠["fill":¯1] x  ⍝ ["i":5]ρ[¯1 10 20 30 40]ₓ
```

Assignment through `⌷` replaces the selected items. A space before `←`
makes the whole run before it the target, as parentheses would. A
missing key is added, even on an axis with no keys.

``` bpl
x←10 20 30
¯1⌷x ←9
x                  ⍝ 10 20 9
T←"aa" "bb":1 2
"cc"⌷T ←3
T                  ⍝ "aa" "bb" "cc":1 2 3
```

`⌷⍠K` applies the items of `I` to the axes `K`, one axis for each
position or key an item holds. `K` lists every axis that a selector of
coordinates covers, in any order, and the selector’s result takes the
place of the lowest of those axes. A named selector listed for an axis
with another name replaces that axis, and the result axis takes the
selector’s name. An axis of `Y` that keeps a selector’s name pairs with
it, which reads one item from each row, as NumPy’s `take_along_axis`
does.

``` bpl
1⌷₁3 4ρι12ₓ           ⍝ [1 5 9]ₓ
⊂[[0 1] [1 2]] (⌷₀ ₂) 2 3 4ρι24ₓ    ⍝ [1 5 9◇14 18 22]ₓ
m←["r":3 "c":3]ρι9ₓ ◇ p←["r":3]ρ2 0 1
[p]⌷⍠"c" m         ⍝ ["r":3]ρ[2 3 7]ₓ
```

## Inverse

`⌷⁻Y` is `⌷Y`.

``` bpl
⌷⁻1 2   ⍝ 1 2
```

## Errors

- `LENGTH`: selectors that share a name but differ in length,
  coordinates that end at cells of different shapes, or suffixes that
  select different shapes in them
- `INDEX`: selectors consuming more axes than the array has, two
  selectors for one axis, a named selector with no axis to select
- `DOMAIN`: coordinates of differing lengths beside another selector
