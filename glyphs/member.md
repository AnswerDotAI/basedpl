

# `∊` — Enlist / Membership

Keys: `Alt-e`. Ranks: `∞` monadic, `∞ ∞` dyadic

`∊Y` recursively ravels all nested leaves.

``` bpl
∊[[1 2] 3 [4 5]]    ⍝ 1 2 3 4 5
```

`X∊Y` marks each cell of `X` found among the major cells of `Y`, using
tolerant matching. The cells of `X` have the rank of a major cell of
`Y`. Against a vector, each item of `X` is one query. A single query
returns an atom. Batch axes follow the leading axes of `X`.

``` bpl
1 2 3∊2 4          ⍝ $f $t $f
"banana"∊"an"       ⍝ $f $t $t $t $t $t
C←"cat" "dog"
⊂"dog" ∊ C         ⍝ $t
["dog"]∊C          ⍝ [$t]
```

Against a matrix, each row of `X` is one query.

``` bpl
m←[10 20⋄30 40]
30 40∊m            ⍝ $t
[30 40⋄50 60]∊m  ⍝ $t $f
```

## Errors

- `RANK`: `Y` is a unit, or `X` has lower rank than a major cell of `Y`
- `LENGTH`: the trailing axes of `X` differ from the shape of a major
  cell of `Y`
