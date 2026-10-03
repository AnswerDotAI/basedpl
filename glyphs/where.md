

# `⍸` — Where / Interval index

Keys: `Alt-i _`

`⍸Y` lists exact positions, repeated according to their nonnegative
integer counts. Higher-rank arguments give coordinate vectors.

``` bpl
⍸0 2 0 1           ⍝ [1 1 3]ₓ
```

For each item of `Y`, `X⍸Y` counts the boundaries in `X` less than or
equal to that item, by binary search. `X` must be sorted. Unsorted
boundaries give meaningless counts, not an error. It uses [structural
ordering](../evaluation.qmd#equality-and-ordering), without tolerance.

``` bpl
10 20 30⍸5 10 25 40 ⍝ [0 1 2 3]ₓ
1 3⍸2              ⍝ 1ₓ
1 3⍸⊂2             ⍝ 1ₓ
1 3⍸,2             ⍝ [1]ₓ
```

Query cells and batch axes follow [Index Of](iota.qmd).
