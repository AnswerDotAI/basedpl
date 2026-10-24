

# `•index` — Fresh indices

`•index n` gives `⍳n` along an axis with a fresh name. `stem •index n`
starts the name with the text `stem`. The name is `_`, the stem, `·` and
a random part of 64 bits written as 13 digits, as in `_b·k3f9q2m7x1c0p`.
Two calls give different names, except with a chance of about one in
10¹⁹. `•index` maps over an array of numbers, with the same stem for
each.

A name that starts with `_` is hidden. Plain display leaves hidden axis
names out. Any key or name that starts with `_` and contains `·`
displays up to its last `·`, then `…`. Box mode and `⍴` show a fresh
name as `_b·…`. To keep a long key or name of your own short on screen,
start it with `_` and put a `·` before the part to leave out:
`["_note·long description":3]` displays as `["_note·…":3]`. `•literal⁻¹`
writes every name in full. A hidden name meets a visible one as if that
axis had no name, and the result keeps the visible name:

``` bpl
i←•index 3 ⋄ v←⍳["v":3]
i          ⍝ 0 1 2
⍴i         ⍝ ["_·…":3]ₓ
⍴i+v       ⍝ ["v":3]ₓ
```

Indices with different names build a grid, in arithmetic and in
selection. An index used twice pairs with itself. Here `x.[i]-x.[j]`
holds the difference between every pair of items of `x`, and `m.[i i]`
is the diagonal of `m`:

``` bpl
x←10 20 40 ⋄ [i j]←•index 3 3
⌈/,|x.[i]-x.[j]      ⍝ 30
m←[1 2 3⋄4 5 6⋄7 8 9]
+/m.[i i]            ⍝ 15
```

Indices name the axes of a formula, as einsum’s letters do. In this
product, `k` pairs by name and [`+/⍠k`](../glyphs/quad-colon.qmd) sums
along it. `b` and `o` build the result’s grid.
[`⍉⁻¹`](../glyphs/circle-backslash.qmd) reorders its axes by name:

``` bpl
xu←[2 3]⍴⍳6ₓ ⋄ wu←[3 4]⍴⍳12ₓ
[b o k]←•index 2 4 3
y←+/⍠k xu.[b k]×wu.[k o]
∧/,y=xu+.×wu                 ⍝ $t
∧/,([o b]⍉⁻¹ y)=⍉xu+.×wu     ⍝ $t
```

Names from `•index` can’t collide with the names on a caller’s data.
[`⍳["i":n]`](../glyphs/iota.qmd) gives a name that the caller chooses.

Errors: `DOMAIN` for a stem that isn’t text.
