

# [`ρ`](https://answerdotai.github.io/basedpl/glyphs/rho.html) — Shape / Reshape

Function. Keys: `Alt-r`. Hanzi: `形`. Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `1 ∞` dyadic

`ρY` gives axis lengths.

``` bpl
ρ2 3ρ0             ⍝ [2 3]ₓ
```

The shape is keyed by the axis names. The length of an unnamed axis has
no key. See [named axes](../keyed.ipynb#named-axes).

``` bpl
M←["city":2 "month":3]ρι6ₓ
ρM                 ⍝ ["city":2 "month":3]ₓ
"month"⌷ρM         ⍝ 3ₓ
```

`SρY` reshapes, cycling or truncating the items of [`,Y`](comma.qmd).
Items of `S` are nonnegative [whole](../terms.qmd#term-whole) numbers.
With empty `Y`, every item is fill, which [`⍠`](quad-colon.qmd#fill) can
set.

``` bpl
2 3ρ1 2            ⍝ [1 2 1◇2 1 2]
3ρ""               ⍝ "   "
```

With empty `S`, the result is a [scalar](../terms.qmd#term-scalar).

``` bpl
⍬ρ1 2              ⍝ ⊂1
```

The keys of `S` name the result’s axes. An unkeyed item leaves its axis
unnamed. When `S` matches the shape of `Y`, the result keeps the keys of
`Y`.

``` bpl
ρ["city":2 3]ρι6ₓ   ⍝ ["city":2 3]ₓ
```

## Inverse

`Sρ⁻Y` gives [`,Y`](comma.qmd). The axis lengths of `Y` must match `S`.

``` bpl
2 3ρ⁻2 3ρι6ₓ   ⍝ [0 1 2 3 4 5]ₓ
```

## Errors

- `DOMAIN`: an item of `S` isn’t a nonnegative whole number, or, for
  `ρ⁻`, the shape of `Y` isn’t `S`
- `LIMIT`: the result has too many items
