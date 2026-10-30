

# Models in index notation

Index notation writes the formula for one item of a result, with a named
index for each of its axes. BPL runs the formula once, on whole arrays,
and `∂` differentiates it. Tacit code stays BPL’s main style: a plain
matrix product reads better as `x+.×w` than in any index form. Index
notation helps where leading axes and Rank line up a formula’s axes
badly, as in attention’s heads and causal mask, a convolution’s stride
and window, or one step of Gaussian elimination.

## Indices

A formula in index notation takes three steps:

1.  Make the indices. [`•index`](system/index.qmd) gives `⍳n` along a
    fresh hidden axis name. It maps over a nested shape, so
    `[[n i] h]←•index[⍴x ≢b]` makes an index for each axis of `x` and
    one for the length of `b`.
2.  Select with them. `x.[n i]` holds the items of `x` along axes named
    by `n` and `i`.
3.  Name the axis that each fold removes, as in `+/⍠i`.

Indices pair by name. The same index pairs with itself. Different
indices build a grid. A matrix product sums along the index that its two
operands share. Here `k` pairs the columns of `xu` with the rows of
`wu`, and `+/⍠k` sums along it. `b` and `o` build the result’s grid. The
[inner product](glyphs/dot.qmd) along `k`, `+.×⍠k`, writes the same
contraction. The examples below use it:

``` bpl
xu←[2 3]⍴⍳6ₓ ⋄ wu←[3 4]⍴⍳12ₓ ⋄ [b o k]←•index 2 4 3
y←+/⍠k⁝xu.[b k]×wu.[k o]
∧/,y=xu+.×wu                   ⍝ $t
∧/,y=xu.[b k]+.×⍠k⁝wu.[k o]     ⍝ $t
```

Each index has its length from the start. BPL doesn’t infer a range from
the reads that use it.

## A neural network

This network has one hidden layer of tanh units (`7○`) and fits `y` from
`x` by mean squared error. `p` holds its parameters. `mlp` is the tacit
form, where `b(+⍤1)` adds the bias to each row. In the index form,
`mlpi`, `n` names the samples, `i` the inputs and `h` the hidden units.
Each line of `mlpi` gives its formula for one sample and one unit. `a`
is the tanh of the bias at `h` plus the sum over `i` of `x` at `n i`
times `w` at `i h`. Both forms give the same loss and the same gradient:

``` bpl
•prefs ["seed":42]
x←¯0.5+¿[64 8]⍴0 ⋄ y←1○+/x
p←["w":0.5×¯0.5+¿[8 16]⍴0 "b":16⍴0 "v":0.1×¯0.5+¿16⍴0 "c":0]
mlp←{
  [x y]←⍺ ⋄ [w b v c]←⍵
  h←7○b(+⍤1)x+.×w
  +/÷≢ ⊼y-c+h+.×v
}
[x y]mlp p                     ⍝ 0.3179859389
mlpi←{
  [x y]←⍺ ⋄ [w b v c]←⍵
  [[n i] h]←•index[⍴x ≢b]
  a←7○b.[h]+x.[n i]+.×⍠i⁝w.[i h]
  +/÷≢ ⊼y.[n]-c+a+.×⍠h⁝v.[h]
}
[x y]mlpi p                    ⍝ 0.3179859389
g←[x y]mlp∂ p ⋄ gi←[x y]mlpi∂ p
1E¯12>⌈/,|g.w-gi.w             ⍝ $t
```

The index form runs at about the tacit form’s speed, forward and under
`∂`. BPL reads `x.[n i]` through its indices as a map, without copying
`x`. An inner product along a named axis, as in `x.[n i]+.×⍠i⁝w.[i h]`,
runs as the matrix product `x+.×w`. So does a `+` fold along a named
axis of a product, as in `+/⍠i⁝x.[n i]×w.[i h]`. Neither builds the grid
of products. The folds in the gradient run as products too. A traced
program also stores each map in the order of axes that most of its
readers need.

## Attention

Attention lets each token of a sequence read from the tokens before it.
`x` holds a batch of 2 sequences of 4 tokens, each with 8 features.
`wqkv` projects each token to a query, a key and a value for each of 2
heads, with 4 features in each head. `wo` projects the heads back to 8
features. `mask` stops a token from reading the tokens after it.

In tacit form, two transposes and the reshape in `heads` line the axes
up for `+.×` and Rank. In index notation, `b` names the sequence, `t`
the token, `d` the feature and `u` the token that `t` reads from. `r`
picks the query, key or value, `h` names the head and `j` the feature
within a head. Both forms give the same result and the same gradient:

``` bpl
x←¯0.5+¿[2 4 8]⍴0 ⋄ wqkv←0.3×¯0.5+¿[8 3 2 4]⍴0 ⋄ wo←0.3×¯0.5+¿[8 8]⍴0
mask←¯1E9× ⍳4 <⊗ ⍳4
heads←{h←0 2 1 3⍉⍵ ⋄ (¯2↓⍴h , ×/¯2↑⍴h)⍴h}
attn←{
  [wqkv wo]←⍺
  [q k v]←1 3 0 2 4⍉⍵+.×wqkv
  s←mask(+⍤2) q+.×⍤2[0 1 3 2]⍉k ÷ √⊢/⍴q
  e←*s-⌈/s
  (heads e÷+/e +.×⍤2 v)+.×wo
}
attni←{
  [wqkv wo]←⍺
  [[b t d] u [r h j]]←•index[⍴⍵ (⍴⍵)₁ 1↓⍴wqkv]
  [q k v]←:[r b t h j]⍉⁻¹⍵.[b t d]+.×⍠d⁝wqkv.[d r h j]
  s←mask.[t u] + q.[b t h j]+.×⍠j⁝k.[b u h j] ÷ √≢j
  e←*s-⌈/⍠u⁝s
  o←(e÷+/⍠u⁝e)+.×⍠u⁝v.[b u h j]
  :[b t d]⍉⁻¹+/⍠[h j]o×wo.[j+(≢j)×h d]
}
a←[wqkv wo]attn x ⋄ ai←[wqkv wo]attni x
1E¯12>⌈/,|a-ai                 ⍝ $t
pa←["wqkv":wqkv "wo":wo]
g←{+/, ⊼[⍵.wqkv ⍵.wo]attn⁝x}∂ pa ⋄ gi←{+/, ⊼[⍵.wqkv ⍵.wo]attni⁝x}∂ pa
1E¯12>⌈/,|g.wqkv-gi.wqkv       ⍝ $t
```

Line by line, in `attni`:

- The projection is one contraction over `d`.
  [`⍉⁻¹`](glyphs/circle-backslash.qmd) puts `r` first by name.
  [`:`](glyphs/colon.qmd) removes the names. `[q k v]←` then takes the
  three parts along the first axis. Without names, each part can be read
  under new indices.
- `k.[b u h j]` reads the keys with `u` for their token axis. The scores
  then pair each query token `t` with each key token `u`. `mask.[t u]`
  adds ¯1E9 wherever `u` comes after `t`.
- `e÷+/⍠u⁝e` is the softmax over `u`. `o` sums the values over `u`, each
  weighted by its share.
- `wo.[j+(≢j)×h d]` reads the row of `wo` for head `h` and feature `j`.
  BPL reads through positions computed from indices as a map. The fold
  over `[h j]` merges the heads. `:[b t d]⍉⁻¹` orders the result’s axes
  and removes their names, to give `attn`’s plain shape.

## Why it works this way

No feature of BPL requires names or keys. A function uses them where
they express something. Code written for plain data can give other
values on named or keyed data, because there the names say which axes
align.

`•index` gives each index a hidden name that is new on every call. A
caller’s names can’t collide with it. An index used twice pairs with
itself and gives a diagonal, as in `m.[i i]`. Different names build a
grid in any arithmetic, with no `⊗`. A length-1 axis expands to match,
as elsewhere in BPL. A design where names only labelled axes, and only
`⊗` built a grid, would break both. BPL also has no constructor that
gives each index an axis by its position in the call, as NumPy’s `ix_`
does. `•index` builds the same grids by name.

Each, Rank and Stencil (`⌺`) need no new syntax. They suit a function
written for one cell, including one that branches cell by cell. Index
notation suits a formula written for one item, where the axes that pair
differ from line to line. BPL’s design followed a close study of Dex,
whose index sets informed `•index`.

## Not built yet

[Automatic differentiation](autodiff.qmd#not-built-yet) lists what
traced programs don’t cover yet. Two of its items apply to models like
these. A program holds one call of `∂`: a training step’s update and the
loop around it run outside the program. A per-item branch, such as a
selector that Agenda (`⍚`) computes from differentiated values, stops a
trace.
