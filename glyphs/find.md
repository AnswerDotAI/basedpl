

# `⍷` — Groups / Find

Keys: `Alt-Minus e`. Ranks: `∞` monadic, `∞ ∞` dyadic

`⍷Y` gives the positions of each distinct major cell of `Y`, in the
order of [`∪`](union.qmd). It matches cells as `∪` does. On a keyed
axis, the positions are keys. In Dyalog, this is `{⊂⍵}⌸Y`.

``` bpl
⍷"abca"             ⍝ (0 3⋄1⋄2)ₓ
```

`X⍷Y` marks starting positions where the whole pattern `X` occurs in
`Y`. The result has `Y`’s shape. Overlaps count.

``` bpl
"ana"⍷"banana"      ⍝ $f $t $f $t $f $f
1 1⍷1 1 1          ⍝ $t $t $f
```

Works with multidimensional patterns and tolerant comparison of items.
