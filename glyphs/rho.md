

# `⍴` — Shape / Reshape

Function. Keys: `Alt-r`. Hanzi: `形`. Ranks: `∞` monadic, `1 ∞` dyadic

`⍴Y` gives axis lengths.

``` bpl
⍴2 3⍴0             ⍝ [2 3]ₓ
```

The shape is keyed by the axis names. An unnamed axis gives an unkeyed
entry. See [named axes](../keyed.ipynb#named-axes).

``` bpl
M←["city":2 "month":3]⍴⍳6ₓ
⍴M                 ⍝ ["city":2 "month":3]ₓ
"month"⌷⍴M         ⍝ 3ₓ
```

`S⍴Y` reshapes, cycling or truncating the ravel of `Y`. Dimensions are
nonnegative whole numbers. Empty `Y` supplies fill, which
[`⍠`](quad-colon.qmd#fill) can choose.

``` bpl
2 3⍴1 2            ⍝ [1 2 1⋄2 1 2]
3⍴""               ⍝ "   "
```

An empty shape makes a rank-zero array.

``` bpl
⍬⍴1 2              ⍝ ⊂1
```

A keyed `S` names the axes of the result. An unkeyed `S` gives no names.
When `S` matches the shape of `Y`, the result keeps the position keys of
`Y`.

``` bpl
⍴["city":2 3]⍴⍳6ₓ   ⍝ ["city":2 3]ₓ
```

## Inverse

`S⍴⁻¹Y` ravels `Y`, which must have shape `S`.

``` bpl
2 3⍴⁻¹2 3⍴⍳6ₓ   ⍝ [0 1 2 3 4 5]ₓ
```
