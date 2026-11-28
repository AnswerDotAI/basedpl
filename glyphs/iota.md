

# [`ι`](https://answerdotai.github.io/basedpl/glyphs/iota.html) — Indices / Index of

Function. Keys: `Alt-i`. Hanzi: `位`. Ranks: `1` monadic, `∞ ∞` dyadic

`ιN` generates `0…N-1`. A negative `N` counts down, as J’s `i.` does.
Exact input gives exact coordinates.

``` bpl
ι4ₓ                 ⍝ [0 1 2 3]ₓ
ι3ₓ                ⍝ [0 1 2]ₓ
ι¯3                ⍝ 2 1 0
```

[Range](right-arrow-from-bar.qmd)
[`↦`](https://answerdotai.github.io/basedpl/glyphs/right-arrow-from-bar.html)
counts from 1 instead, or between any two ends.

``` bpl
↦4                 ⍝ 1 2 3 4
3↦6                ⍝ 3 4 5 6
```

Shape vector: an array of coordinate vectors. Negative length: that axis
reversed.

``` bpl
ι2ₓ 2               ⍝ [[0 0] [0 1]◇[1 0] [1 1]]
ι2ₓ ¯2              ⍝ [[0 1] [0 0]◇[1 1] [1 0]]
```

A keyed shape names the result’s axes, as for [Reshape](rho.qmd). The
result is a named index, which pairs by name with axes of the same name.
[Indexing](squad.qmd) by a named index gives the result its name.
[`•index`](../system/index.qmd) gives an index whose name can’t collide
with a caller’s names:

``` bpl
ρι["i":3]          ⍝ ["i":3]ₓ
x←10×ι5ₓ ◇ i←ι["i":5]
x.[5|i+1]          ⍝ ["i":5]ρ[10 20 30 40 0]
```

On a matrix,
[`ι`](https://answerdotai.github.io/basedpl/glyphs/iota.html) applies to
each row, as [Rank](jot-diaeresis.qmd) does. Results are padded with
fill to a common shape.

``` bpl
ρι[2 3◇1 1]       ⍝ [2 2 3]ₓ
```

`ι⍠A Y` returns the keys of axis `A`, or `0…length-1` on an unkeyed
axis. The result’s axis has the selected axis’s name. It pairs with that
axis by name in arithmetic and indexing. Indexing by it keeps the keys
at the positions it selects. Several axes: a vector of selector vectors.

``` bpl
ι₀["price":1 "qty":2]   ⍝ "price" "qty"
ι₁2 3ρ0              ⍝ [0 1 2]ₓ
ρι⍠"j" ["i":2 "j":3]ρ0 ⍝ ["j":3]ₓ
```

On an axis where only some positions have keys, `ι⍠A Y` gives the keys
where present and positions elsewhere.

``` bpl
ι₀["aa":10 20]      ⍝ "aa" 1ₓ
```

`XιY` finds the first matching major cell in `X` for each cell of `Y`.
Not found: `≢X`. Uses
[tolerant](../evaluation.qmd#equality-and-ordering) matching and returns
exact positions.

``` bpl
"abc"ι"cabz"        ⍝ [2 0 1 3]ₓ
"abc"ι'b'           ⍝ 1ₓ
"abc"ι⊂'b'          ⍝ 1ₓ
```

Each cell of `Y` with the rank of `X`’s major cells is one query. One
query returns an atom; otherwise the result has the frame’s shape. To
search for an array as one item, enclose it with
[`⊂`](https://answerdotai.github.io/basedpl/glyphs/left-shoe.html).

``` bpl
C←"cat" "dog"
Cι⊂"dog"           ⍝ 1ₓ
Cι["dog"]          ⍝ [1]ₓ
m←[10 20◇30 40◇50 60]
mι30 40            ⍝ 1ₓ
mι[30 40◇]        ⍝ [1]ₓ
mι[30 40◇10 20]  ⍝ [1 0]ₓ
```

## Inverse

`ι⁻Y` gives the argument that generates `Y`: the count for a vector of
positions, otherwise the shape. `Xι⁻Y` gives the major cells of `X` at
the positions `Y`. For a vector of atoms `X`, position `≢X` gives fill,
unless `X` holds its fill. Any other position without a cell raises
`DOMAIN`.

``` bpl
ι⁻ι5         ⍝ [5]ₓ
"abc"ι⁻2 0   ⍝ "ca"
```
